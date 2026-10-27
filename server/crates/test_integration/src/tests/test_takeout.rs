use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use app_state::constants::TUS_UPLOADS_FOLDER;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use color_eyre::Result;
use color_eyre::eyre::{bail, eyre};
use reqwest::StatusCode;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use zip::write::SimpleFileOptions;

/// A valid 8x8 PNG without any capture metadata, so the import has to take its date from the sidecar.
const NO_METADATA_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x08, 0x06, 0x00, 0x00, 0x00, 0xc4, 0x0f, 0xbe,
    0x8b, 0x00, 0x00, 0x00, 0x01, 0x73, 0x52, 0x47, 0x42, 0x00, 0xae, 0xce, 0x1c, 0xe9, 0x00, 0x00,
    0x00, 0x04, 0x67, 0x41, 0x4d, 0x41, 0x00, 0x00, 0xb1, 0x8f, 0x0b, 0xfc, 0x61, 0x05, 0x00, 0x00,
    0x00, 0x09, 0x70, 0x48, 0x59, 0x73, 0x00, 0x00, 0x0e, 0xc3, 0x00, 0x00, 0x0e, 0xc3, 0x01, 0xc7,
    0x6f, 0xa8, 0x64, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x28, 0x53, 0x63, 0x60, 0x18,
    0x05, 0x20, 0x00, 0x00, 0x01, 0x08, 0x00, 0x01, 0xa9, 0x53, 0x7b, 0x0d, 0x00, 0x00, 0x00, 0x00,
    0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

const EXISTING_VIDEO: &str = "VID_20190223_155900.mp4";
const NEW_PHOTO: &str = "takeout_new(1).png";
const SIDECAR_TIMESTAMP: i64 = 1_690_000_000; // 2023-07-22 04:26:40 UTC
const BROKEN_PHOTO: &str = "takeout_broken(1).jpg";
const BROKEN_SIDECAR_TIMESTAMP: i64 = 1_690_000_060; // 2023-07-22 04:27:40 UTC

fn build_zip(path: &Path, entries: &[(&str, &[u8])]) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(*name, SimpleFileOptions::default())?;
        zip.write_all(content)?;
    }
    zip.finish()?;
    Ok(())
}

/// Takeout spreads one album over multiple ZIPs, so the archives together form one import batch.
fn create_takeout_zips(context: &TestContext, folder: &Path) -> Result<(PathBuf, PathBuf)> {
    let video = std::fs::read(context.settings.ingest.media_root.join(EXISTING_VIDEO))?;
    let sidecar = format!(
        r#"{{"title": "{NEW_PHOTO}", "photoTakenTime": {{"timestamp": "{SIDECAR_TIMESTAMP}"}}}}"#
    );
    let sidecar = sidecar.as_bytes();
    let broken_sidecar = format!(
        r#"{{"title": "{BROKEN_PHOTO}", "photoTakenTime": {{"timestamp": "{BROKEN_SIDECAR_TIMESTAMP}"}}}}"#
    );
    let broken_sidecar = broken_sidecar.as_bytes();
    let album_metadata = br#"{"title": "Summer Vacation", "description": "Summer holiday"}"#;

    let zip1 = folder.join("takeout-001.zip");
    build_zip(
        &zip1,
        &[
            (
                "Takeout/Google Photos/Vacation 2023 /metadata.json",
                album_metadata,
            ),
            (
                "Takeout/Google Photos/Vacation 2023 /takeout_new(1).png",
                NO_METADATA_PNG,
            ),
            (
                "Takeout/Google Photos/Vacation 2023 /takeout_broken(1).jpg",
                NO_METADATA_PNG,
            ),
            (
                "Takeout/Google Photos/Vacation 2023 /takeout_broken.jpg.supplemental-metadata(1).json",
                broken_sidecar,
            ),
            (
                "Takeout/Google Photos/Vacation 2023 /VID_20190223_155900.mp4",
                &video,
            ),
        ],
    )?;

    let zip2 = folder.join("takeout-002.zip");
    build_zip(
        &zip2,
        &[
            (
                "Takeout/Google Photos/Vacation 2023 /takeout_new.png.supplemental-metadata(1).json",
                sidecar,
            ),
            (
                "Takeout/Google Photos/Photos from 2023 /takeout_new(1).png",
                NO_METADATA_PNG,
            ),
            (
                "Takeout/Google Photos/Photos from 2023 /takeout_new.png.supplemental-metadata(1).json",
                sidecar,
            ),
            (
                "Takeout/Google Photos/Photos from 2023 /takeout_broken(1).jpg",
                NO_METADATA_PNG,
            ),
            (
                "Takeout/Google Photos/Photos from 2023 /takeout_broken.jpg.supplemental-metadata(1).json",
                broken_sidecar,
            ),
            (
                "Takeout/Google Photos/Photos from 2023 /VID_20190223_155900.mp4",
                &video,
            ),
        ],
    )?;

    Ok((zip1, zip2))
}

async fn tus_create(
    client: &reqwest::Client,
    api_url: &str,
    upload_jwt: &str,
    filename: &str,
    metadata: &[(&str, &str)],
    size: usize,
) -> Result<(String, String)> {
    let mut pairs = vec![
        format!("filename {}", STANDARD.encode(filename)),
        format!("jwt {}", STANDARD.encode(upload_jwt)),
    ];
    pairs.extend(
        metadata
            .iter()
            .map(|(key, value)| format!("{key} {}", STANDARD.encode(value))),
    );

    let create = client
        .post(format!("{api_url}/api/files"))
        .header("Tus-Resumable", "1.0.0")
        .header("Upload-Length", size.to_string())
        .header("Upload-Metadata", pairs.join(","))
        .send()
        .await?;

    if create.status() != StatusCode::CREATED {
        bail!(
            "Tus create failed: {} {}",
            create.status(),
            create.text().await?
        );
    }

    let location = create
        .headers()
        .get("Location")
        .ok_or_else(|| eyre!("Tus create response is missing the Location header"))?
        .to_str()?
        .to_owned();
    let upload_id = location.rsplit('/').next().unwrap_or_default().to_owned();

    Ok((location, upload_id))
}

async fn tus_patch(
    client: &reqwest::Client,
    location: &str,
    offset: u64,
    bytes: &[u8],
) -> Result<()> {
    let patch = client
        .patch(location)
        .header("Tus-Resumable", "1.0.0")
        .header("Upload-Offset", offset.to_string())
        .header("Content-Type", "application/offset+octet-stream")
        .body(bytes.to_vec())
        .send()
        .await?;

    if patch.status() != StatusCode::NO_CONTENT {
        bail!(
            "Tus upload failed: {} {}",
            patch.status(),
            patch.text().await?
        );
    }

    Ok(())
}

/// Minimal tus creation + upload, so the test exercises the real upload-finished batch detection.
async fn tus_upload(
    client: &reqwest::Client,
    api_url: &str,
    upload_jwt: &str,
    filename: &str,
    metadata: &[(&str, &str)],
    bytes: &[u8],
) -> Result<String> {
    let (location, upload_id) =
        tus_create(client, api_url, upload_jwt, filename, metadata, bytes.len()).await?;
    tus_patch(client, &location, 0, bytes).await?;
    Ok(upload_id)
}

/// Leaves an upload in the state produced by a browser refresh during finalization:
/// `.info` says complete, the final file is a prefix, and the parts hold the full ZIP.
fn simulate_interrupted_finalize(tus_dir: &Path, upload_id: &str, bytes: &[u8]) -> Result<()> {
    let split = bytes.len() / 2;
    std::fs::write(tus_dir.join(format!("{upload_id}_part_0")), &bytes[..split])?;
    std::fs::write(tus_dir.join(format!("{upload_id}_part_1")), &bytes[split..])?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(tus_dir.join(upload_id))?
        .set_len(u64::try_from(split / 2)?)?;
    Ok(())
}

async fn start_takeout_import(client: &reqwest::Client, api_url: &str) -> Result<(bool, usize)> {
    let response = client
        .post(format!("{api_url}/api/upload/takeout/start"))
        .send()
        .await?;

    if !response.status().is_success() {
        bail!(
            "Starting the takeout import failed: {} {}",
            response.status(),
            response.text().await?
        );
    }

    let body: serde_json::Value = response.json().await?;
    Ok((
        body.get("started")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        usize::try_from(
            body.get("zipCount")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
    ))
}

async fn wait_for_takeout_job(context: &TestContext) -> Result<()> {
    let timeout = Duration::from_mins(2);
    let start = Instant::now();

    loop {
        let status: Option<String> = sqlx::query_scalar!(
            r#"SELECT status::text AS "status!" FROM jobs WHERE job_type = 'ingest_takeout_batch' ORDER BY id DESC LIMIT 1"#
        )
        .fetch_optional(&context.pool)
        .await?;

        match status.as_deref() {
            Some("done") => return Ok(()),
            Some("failed") => {
                let error: Option<String> = sqlx::query_scalar!(
                    "SELECT last_error FROM jobs WHERE job_type = 'ingest_takeout_batch' ORDER BY id DESC LIMIT 1"
                )
                .fetch_one(&context.pool)
                .await?;
                bail!("Takeout import job failed: {}", error.unwrap_or_default());
            }
            _ => {}
        }

        if start.elapsed() > timeout {
            bail!("Timed out waiting for the takeout import job (last status: {status:?})");
        }
        sleep(Duration::from_millis(500)).await;
    }
}

async fn wait_for_media_item(
    context: &TestContext,
    user_id: i32,
    filename: &str,
) -> Result<(String, String, NaiveDateTime)> {
    let timeout = Duration::from_mins(1);
    let start = Instant::now();

    loop {
        let item = sqlx::query!(
            r#"
            SELECT id, relative_path, taken_at_local
            FROM media_item
            WHERE user_id = $1 AND filename = $2 AND deleted = false
            "#,
            user_id,
            filename
        )
        .fetch_optional(&context.pool)
        .await?;

        if let Some(item) = item {
            return Ok((item.id, item.relative_path, item.taken_at_local));
        }
        if start.elapsed() > timeout {
            bail!("Timed out waiting for '{filename}' to be imported");
        }
        sleep(Duration::from_millis(500)).await;
    }
}

async fn assert_album_linking(
    context: &TestContext,
    user_id: i32,
    new_media_id: &str,
) -> Result<()> {
    let album_ids: Vec<String> = sqlx::query_scalar!(
        "SELECT id FROM album WHERE owner_id = $1 AND name = ANY($2)",
        user_id,
        &["Summer Vacation".to_owned(), "Photos from 2023".to_owned()]
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(
        album_ids.len(),
        2,
        "both Takeout folders should have become albums, using metadata.json's title"
    );
    let sorts: Vec<String> = sqlx::query_scalar!(
        r#"SELECT sort_mode::text AS "sort_mode!" FROM album WHERE id = ANY($1::TEXT[])"#,
        &album_ids
    )
    .fetch_all(&context.pool)
    .await?;
    assert!(
        sorts.iter().all(|sort| sort == "date_asc"),
        "Takeout albums must not be manually sorted"
    );

    for album_id in &album_ids {
        let in_album = sqlx::query_scalar!(
            r#"SELECT EXISTS (
                SELECT 1 FROM album_media_item WHERE album_id = $1 AND media_item_id = $2
            ) AS "exists!""#,
            album_id,
            new_media_id
        )
        .fetch_one(&context.pool)
        .await?;
        assert!(
            in_album,
            "the new photo should be in every Takeout album that contains it"
        );

        let owner_role: Option<String> = sqlx::query_scalar!(
            r#"SELECT role::text AS "role!" FROM album_collaborator WHERE album_id = $1 AND user_id = $2"#,
            album_id,
            user_id
        )
        .fetch_optional(&context.pool)
        .await?;
        assert_eq!(
            owner_role.as_deref(),
            Some("owner"),
            "the Takeout album owner must have the owner collaborator row"
        );
    }

    let album_description: Option<String> = sqlx::query_scalar!(
        "SELECT description FROM album WHERE owner_id = $1 AND name = 'Summer Vacation'",
        user_id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(album_description.as_deref(), Some("Summer holiday"));

    Ok(())
}

async fn assert_hash_matches_file(
    context: &TestContext,
    media_id: &str,
    relative_path: &str,
) -> Result<()> {
    let stored_hash: String =
        sqlx::query_scalar!("SELECT hash FROM media_item WHERE id = $1", media_id)
            .fetch_one(&context.pool)
            .await?;
    let disk_hash = common_services::caching::hash_file(
        &context.settings.ingest.media_root.join(relative_path),
    )?;
    assert_eq!(
        stored_hash, disk_hash,
        "the stored hash should match the file after the EXIF datetime write"
    );
    Ok(())
}

async fn assert_existing_media_linking(context: &TestContext, user_id: i32) -> Result<()> {
    let existing_items: Vec<String> = sqlx::query_scalar!(
        "SELECT id FROM media_item WHERE user_id = $1 AND filename = $2",
        user_id,
        EXISTING_VIDEO
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(
        existing_items.len(),
        1,
        "already imported media must not be imported again"
    );

    let photos_from_2023_id: String = sqlx::query_scalar!(
        "SELECT id FROM album WHERE owner_id = $1 AND name = 'Photos from 2023'",
        user_id
    )
    .fetch_one(&context.pool)
    .await?;
    let existing_linked = sqlx::query_scalar!(
        r#"SELECT EXISTS (
            SELECT 1 FROM album_media_item WHERE album_id = $1 AND media_item_id = $2
        ) AS "exists!""#,
        photos_from_2023_id,
        existing_items[0]
    )
    .fetch_one(&context.pool)
    .await?;
    assert!(
        existing_linked,
        "already imported media should still be added to the Takeout album"
    );
    assert!(
        !context
            .settings
            .ingest
            .media_root
            .join("takeout")
            .join(EXISTING_VIDEO)
            .exists(),
        "already imported media must not be copied into the takeout folder"
    );

    Ok(())
}

fn assert_cleanup(context: &TestContext, job_id: i64, upload_ids: &[String]) {
    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);
    for upload_id in upload_ids {
        assert!(!tus_dir.join(upload_id).exists(), "ZIP should be deleted");
        assert!(
            !tus_dir.join(format!("{upload_id}.info")).exists(),
            "ZIP info file should be deleted"
        );
    }
    assert!(
        !context
            .settings
            .ingest
            .cache_root
            .join("takeout_tmp")
            .join(job_id.to_string())
            .exists(),
        "extraction folder should be deleted"
    );
}

#[allow(clippy::too_many_lines)]
pub async fn test_takeout_import_flow(context: &TestContext) -> Result<()> {
    login(context).await?;
    let api_url = &context.settings.api.public_url;
    let client = &context.http_client;

    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let upload_jwt: String = client
        .get(format!("{api_url}/api/upload/jwt"))
        .send()
        .await?
        .json()
        .await?;

    let temp_dir = tempfile::TempDir::new()?;
    let (zip1_path, zip2_path) = create_takeout_zips(context, temp_dir.path())?;
    let metadata = [("isTakeout", "true")];

    let zip1_bytes = std::fs::read(&zip1_path)?;
    let zip2_bytes = std::fs::read(&zip2_path)?;

    // Simulate an interrupted upload: zip2 is created and half uploaded, then the
    // browser is refreshed. The client only calls start once its queue is done.
    let (zip2_location, zip2_id) = tus_create(
        client,
        api_url,
        &upload_jwt,
        "takeout-002.zip",
        &metadata,
        zip2_bytes.len(),
    )
    .await?;
    let split = zip2_bytes.len() / 2;
    tus_patch(client, &zip2_location, 0, &zip2_bytes[..split]).await?;

    let (started, zip_count) = start_takeout_import(client, api_url).await?;
    assert!(!started, "partial ZIPs must not start an import");
    assert_eq!(zip_count, 0);

    // The already-finished ZIP resumes without another upload event; the partial one
    // is finished after the refresh.
    let zip1_id = tus_upload(
        client,
        api_url,
        &upload_jwt,
        "takeout-001.zip",
        &metadata,
        &zip1_bytes,
    )
    .await?;
    tus_patch(client, &zip2_location, split as u64, &zip2_bytes[split..]).await?;

    // A refresh during the final PATCH leaves .info complete but the final file truncated.
    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);
    simulate_interrupted_finalize(&tus_dir, &zip1_id, &zip1_bytes)?;
    assert!(
        std::fs::metadata(tus_dir.join(&zip1_id))?.len() < u64::try_from(zip1_bytes.len())?,
        "the upload should start out truncated"
    );

    let (started, zip_count) = start_takeout_import(client, api_url).await?;
    assert!(started);
    assert_eq!(zip_count, 2);
    assert_eq!(
        std::fs::read(tus_dir.join(&zip1_id))?,
        zip1_bytes,
        "the interrupted finalization should be repaired before enqueueing"
    );
    assert!(!tus_dir.join(format!("{zip1_id}_part_0")).exists());
    assert!(!tus_dir.join(format!("{zip1_id}_part_1")).exists());
    wait_for_takeout_job(context).await?;

    let job_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM jobs WHERE job_type = 'ingest_takeout_batch'")
            .fetch_one(&context.pool)
            .await?
            .expect("count");
    assert_eq!(job_count, 1, "the batch should only be enqueued once");

    let (job_id, payload): (i64, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT id, payload FROM jobs WHERE job_type = 'ingest_takeout_batch' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&context.pool)
    .await?;
    let mut expected_ids = vec![zip1_id.clone(), zip2_id.clone()];
    expected_ids.sort_unstable();
    let payload_ids: Vec<String> = payload
        .and_then(|payload| serde_json::from_value(payload.get("tus_upload_ids")?.clone()).ok())
        .expect("takeout payload should contain the upload ids");
    assert_eq!(payload_ids, expected_ids);

    // The new photo is imported from the takeout folder, with the sidecar date burned into it.
    let (new_media_id, relative_path, taken_at_local) =
        wait_for_media_item(context, user_id, NEW_PHOTO).await?;
    assert_eq!(relative_path, format!("takeout/{NEW_PHOTO}"));
    assert_eq!(
        taken_at_local,
        NaiveDate::from_ymd_opt(2023, 7, 22)
            .unwrap()
            .and_hms_opt(4, 26, 40)
            .unwrap(),
        "the sidecar date should have been written to the file and read back during ingest"
    );

    assert_hash_matches_file(context, &new_media_id, &relative_path).await?;

    let (broken_media_id, broken_relative_path, broken_taken_at_local) =
        wait_for_media_item(context, user_id, BROKEN_PHOTO).await?;

    assert_eq!(broken_relative_path, format!("takeout/{BROKEN_PHOTO}"));
    assert_eq!(
        broken_taken_at_local,
        NaiveDate::from_ymd_opt(2023, 7, 22)
            .unwrap()
            .and_hms_opt(4, 27, 40)
            .unwrap(),
        "the sidecar date must reach the DB even when ExifTool cannot write the file"
    );

    let broken_time = sqlx::query!(
        r#"
        SELECT mi.taken_at_utc, mi.og_taken_at_local,
               mi.timezone_name, mi.timezone_offset_seconds, mi.og_timezone_offset_seconds,
               t.source_details, t.source_confidence
        FROM media_item mi
        JOIN "time" t ON t.media_item_id = mi.id
        WHERE mi.id = $1
        "#,
        broken_media_id
    )
    .fetch_one(&context.pool)
    .await?;

    assert_eq!(
        broken_time.taken_at_utc,
        Some(DateTime::<Utc>::from_timestamp(BROKEN_SIDECAR_TIMESTAMP, 0).unwrap())
    );
    assert_eq!(broken_time.og_taken_at_local, broken_taken_at_local);
    assert_eq!(broken_time.timezone_name.as_deref(), Some("UTC"));
    assert_eq!(broken_time.timezone_offset_seconds, Some(0));
    assert_eq!(broken_time.og_timezone_offset_seconds, Some(0));
    assert_eq!(broken_time.source_details, "Google Takeout sidecar");
    assert_eq!(broken_time.source_confidence, "Medium");

    assert_album_linking(context, user_id, &new_media_id).await?;
    assert_album_linking(context, user_id, &broken_media_id).await?;
    assert_existing_media_linking(context, user_id).await?;
    assert_cleanup(context, job_id, &[zip1_id, zip2_id]);

    let (started, zip_count) = start_takeout_import(client, api_url).await?;
    assert!(!started, "nothing should be left to import after cleanup");
    assert_eq!(zip_count, 0);

    Ok(())
}
