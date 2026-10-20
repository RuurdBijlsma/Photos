use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use app_state::constants::TUS_UPLOADS_FOLDER;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{NaiveDate, NaiveDateTime};
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
                "Takeout/Google Photos/Photos from 2023 /VID_20190223_155900.mp4",
                &video,
            ),
        ],
    )?;

    Ok((zip1, zip2))
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
        .header("Upload-Length", bytes.len().to_string())
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

    let patch = client
        .patch(&location)
        .header("Tus-Resumable", "1.0.0")
        .header("Upload-Offset", "0")
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

    Ok(upload_id)
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

fn assert_cleanup(context: &TestContext, batch_id: &str, upload_ids: &[String]) {
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
            .join(batch_id)
            .exists(),
        "extraction folder should be deleted"
    );
}

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

    let batch_id = "test_takeout_batch";
    let temp_dir = tempfile::TempDir::new()?;
    let (zip1_path, zip2_path) = create_takeout_zips(context, temp_dir.path())?;

    let total_files = "2".to_owned();
    let metadata = [
        ("isTakeout", "true"),
        ("takeoutBatchId", batch_id),
        ("takeoutTotalFiles", total_files.as_str()),
    ];

    let zip1_id = tus_upload(
        client,
        api_url,
        &upload_jwt,
        "takeout-001.zip",
        &metadata,
        &std::fs::read(&zip1_path)?,
    )
    .await?;
    let zip2_id = tus_upload(
        client,
        api_url,
        &upload_jwt,
        "takeout-002.zip",
        &metadata,
        &std::fs::read(&zip2_path)?,
    )
    .await?;

    // The last finished ZIP starts the batch job.
    wait_for_takeout_job(context).await?;

    let job_count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM jobs WHERE job_type = 'ingest_takeout_batch'")
            .fetch_one(&context.pool)
            .await?
            .expect("count");
    assert_eq!(job_count, 1, "the batch should only be enqueued once");

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

    assert_album_linking(context, user_id, &new_media_id).await?;
    assert_existing_media_linking(context, user_id).await?;
    assert_cleanup(context, batch_id, &[zip1_id, zip2_id]);

    Ok(())
}
