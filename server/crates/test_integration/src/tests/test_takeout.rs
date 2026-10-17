use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use color_eyre::Result;
use color_eyre::eyre::{Context, bail, eyre};
use common_types::dev_constants::EMAIL;
use reqwest::StatusCode;
use std::future::Future;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::time::sleep;
use tracing::info;

const BATCH_1: &str = "20261015T162830Z";
const BATCH_2: &str = "20261016T000000Z";
const ZIP_1_NAME: &str = "takeout-20261015T162830Z-1-001.zip";
const ZIP_RENAMED_NAME: &str = "takeout-renamed.zip";

const SOURCE_PHOTO_A: &str = "PXL_20221007_193358147.NIGHT.jpg";
const SOURCE_PHOTO_B: &str = "PXL_20250916_155329360.jpg";
const SOURCE_PHOTO_C: &str = "00000IMG_00000_BURST20190725142501659_COVER.jpg";
/// Filenames unique to this test suite (fixture bytes under fresh names), so
/// the happy-path zip is not deduped against the onboarding-scanned library.
const TAKE_A: &str = "TAKEOUT_A.JPG";
const TAKE_B: &str = "TAKEOUT_B.JPG";
const TAKE_C: &str = "TAKEOUT_C.JPG";
const TESTIMG_A: &str = "TESTIMG_001.JPG";
const TESTIMG_CHUNK_1: &str = "TESTIMG_002.JPG";
const TESTIMG_CHUNK_2: &str = "TESTIMG_003.JPG";

/// Zip root inside an export: Google wraps every product in a `Takeout` folder.
const TAKEOUT_ROOT: &str = "Takeout/Google Photos";

/// Polls until `poll` returns `Some`, bailing with `label` after `timeout`.
async fn wait_for<T, F, Fut>(label: &str, timeout: Duration, mut poll: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<Option<T>>>,
{
    let start = Instant::now();
    loop {
        if let Some(value) = poll().await? {
            return Ok(value);
        }
        if start.elapsed() > timeout {
            bail!("Timed out while waiting for {label}");
        }
        sleep(Duration::from_millis(500)).await;
    }
}

async fn read_source_photo(context: &TestContext, name: &str) -> Result<Vec<u8>> {
    let path = context.settings.ingest.media_root.join(name);
    fs::read(&path)
        .await
        .wrap_err_with(|| format!("reading fixture {path:?}"))
}

fn write_zip(path: &Path, entries: &[(String, Vec<u8>)]) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut writer = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        use std::io::Write;
        writer.start_file(name.clone(), options)?;
        writer.write_all(bytes)?;
    }
    writer
        .finish()
        .wrap_err_with(|| format!("finishing zip {path:?}"))?;
    Ok(())
}

/// Uploads a file through the tus endpoints exactly like the web dropzone does.
/// The upload finishes when the final PATCH is accepted; this enqueues the
/// `HandleWebUpload` job via the tus hook.
async fn upload_file(context: &TestContext, filename: &str, contents: &[u8]) -> Result<()> {
    let api_url = &context.settings.api.public_url;

    let jwt: String = context
        .http_client
        .get(format!("{api_url}/api/upload/jwt"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let metadata = format!(
        "filename {},jwt {}",
        B64.encode(filename.as_bytes()),
        B64.encode(jwt.as_bytes())
    );

    let response = context
        .http_client
        .post(format!("{api_url}/api/files"))
        .header("Tus-Resumable", "1.0.0")
        .header("Upload-Length", contents.len().to_string())
        .header("Upload-Metadata", metadata)
        .send()
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "tus creation failed: {:?}",
        response.text().await?
    );
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .ok_or_else(|| eyre!("tus POST response has no Location header"))?
        .to_str()?
        .to_owned();
    let url = if location.starts_with("http") {
        location
    } else {
        format!("{api_url}{location}")
    };

    let response = context
        .http_client
        .patch(url)
        .header("Tus-Resumable", "1.0.0")
        .header("Upload-Offset", "0")
        .header("Content-Type", "application/offset+octet-stream")
        .body(contents.to_vec())
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    Ok(())
}

async fn user_media_folder(context: &TestContext) -> Result<(i32, String)> {
    struct Row {
        id: i32,
        media_folder: Option<String>,
    }
    let row = sqlx::query_as!(
        Row,
        "SELECT id, media_folder FROM app_user WHERE email = $1",
        EMAIL
    )
    .fetch_one(&context.pool)
    .await?;
    Ok((
        row.id,
        row.media_folder
            .ok_or_else(|| eyre!("user has no media folder yet"))?,
    ))
}

/// Builds the path prefix for a user's library: empty users scan the media
/// root directly, so there is no folder component to prepend.
fn path_prefix(user_media: &str) -> String {
    if user_media.is_empty() {
        String::new()
    } else {
        format!("{user_media}/")
    }
}

/// Number of successfully finished `TakeoutImport` jobs for a batch id.
async fn done_import_count(context: &TestContext, batch_id: &str) -> Result<Option<i64>> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM jobs
        WHERE job_type = 'takeout_import'
          AND status = 'done'
          AND payload->>'batch_id' = $1
        "#,
        batch_id
    )
    .fetch_one(&context.pool)
    .await?;
    Ok(count)
}

/// Waits until `expected` imports of a batch have finished.
async fn wait_import_done(context: &TestContext, batch_id: &str, expected: i64) -> Result<()> {
    info!("Waiting for {expected} takeout_import job(s) of batch {batch_id}...");
    wait_for(
        &format!("takeout_import({batch_id}) x{expected}"),
        Duration::from_mins(4),
        || async {
            Ok((done_import_count(context, batch_id).await? == Some(expected)).then_some(()))
        },
    )
    .await
}

/// Waits until the metadata ingest for a (single) media item path completed;
/// album attachment happens inside that job, so this doubles as the "linked"
/// signal.
async fn wait_item_ingested(context: &TestContext, relative_path: &str) -> Result<()> {
    info!("Waiting for ingest_metadata of {relative_path}...");
    wait_for(
        &format!("ingest_metadata({relative_path})"),
        Duration::from_mins(3),
        || async {
            let count = sqlx::query_scalar!(
                r#"
                SELECT COUNT(*)
                FROM jobs
                WHERE job_type = 'ingest_metadata'
                  AND status = 'done'
                  AND relative_path = $1
                "#,
                relative_path
            )
            .fetch_one(&context.pool)
            .await?;
            Ok((count == Some(1)).then_some(()))
        },
    )
    .await
}

#[allow(dead_code)]
struct AlbumRow {
    id: String,
    name: String,
    description: Option<String>,
    takeout_id: Option<String>,
    media_count: i32,
}

async fn takeout_album(context: &TestContext, name: &str) -> Result<AlbumRow> {
    sqlx::query_as!(
        AlbumRow,
        r#"
        SELECT id, name, description, takeout_id, media_count
        FROM album
        WHERE name = $1 AND takeout_id IS NOT NULL
        "#,
        name
    )
    .fetch_optional(&context.pool)
    .await?
    .ok_or_else(|| eyre!("takeout album {name:?} not found"))
}

async fn all_takeout_albums(context: &TestContext) -> Result<Vec<(String, String)>> {
    let mut rows = sqlx::query!("SELECT name, takeout_id FROM album WHERE takeout_id IS NOT NULL")
        .fetch_all(&context.pool)
        .await?
        .into_iter()
        .map(|r| (r.name, r.takeout_id.expect("filtered above")))
        .collect::<Vec<_>>();
    rows.sort();
    Ok(rows)
}

async fn media_item_ids_for_filename(context: &TestContext, filename: &str) -> Result<Vec<String>> {
    let ids: Vec<String> =
        sqlx::query_scalar!("SELECT id FROM media_item WHERE filename = $1", filename)
            .fetch_all(&context.pool)
            .await?
            .into_iter()
            .collect();
    Ok(ids)
}

/// Album ids a given item is attached to through takeout links (consumed rows),
/// sorted for stable comparison.
async fn takeout_album_ids_for_item(context: &TestContext, item_id: &str) -> Result<Vec<String>> {
    let mut ids: Vec<String> = sqlx::query_scalar!(
        "SELECT DISTINCT album_id FROM takeout_album_link WHERE media_item_id = $1",
        item_id
    )
    .fetch_all(&context.pool)
    .await?
    .into_iter()
    .collect();
    ids.sort();
    Ok(ids)
}

/// All media item paths that live below the takeout folder.
async fn takeout_media_paths(context: &TestContext) -> Result<Vec<String>> {
    let paths: Vec<String> = sqlx::query_scalar!(
        r#"
        SELECT relative_path
        FROM media_item
        WHERE relative_path LIKE '%takeout%'
        ORDER BY relative_path
        "#
    )
    .fetch_all(&context.pool)
    .await?
    .into_iter()
    .collect();
    Ok(paths)
}

/// First happy-path import: albums from folders (with metadata.json title +
/// description), root-level file without album, sidecar files ignored, path
/// layout without `Google Photos` and batch id.
pub async fn test_takeout_import(context: &TestContext) -> Result<()> {
    login(context).await?;
    let user = user_media_folder(context).await?;
    let prefix = path_prefix(&user.1);

    let photo_a = read_source_photo(context, SOURCE_PHOTO_A).await?;
    let photo_b = read_source_photo(context, SOURCE_PHOTO_B).await?;
    let photo_c = read_source_photo(context, SOURCE_PHOTO_C).await?;

    // Build the export zip and stage it where the tus upload will run. We place
    // it under `app_data/takeout/<batch>` so the staging directory naturally
    // exists, then upload its contents through the dropzone pipeline (the
    // upload stages its own copy, which is what the import will process).
    let staging = context
        .settings
        .ingest
        .app_data_root
        .join("takeout")
        .join(BATCH_1);
    std::fs::create_dir_all(&staging)?;
    let staging_zip = staging.join(ZIP_1_NAME);
    write_zip(
        &staging_zip,
        &[
            // Other takeout products must be skipped entirely.
            (
                "Takeout/My Activity/Chrome/History.html".to_owned(),
                b"<html></html>".to_vec(),
            ),
            (
                format!("{TAKEOUT_ROOT}/Italië 2016/metadata.json"),
                r#"{"title":"Get-Italië 2016","description":"Takeout test album","creationTimestamp":{"timestamp":"1759993001"}}"#.as_bytes().to_vec(),
            ),
            (
                format!("{TAKEOUT_ROOT}/Italië 2016/{TAKE_A}"),
                photo_a.clone(),
            ),
            (
                format!("{TAKEOUT_ROOT}/Italië 2016/{TAKE_A}.supplemental-metadata.json"),
                br#"{"title":"x.jpg"}"#.to_vec(),
            ),
            (
                format!("{TAKEOUT_ROOT}/Photos from 2024/{TAKE_B}"),
                photo_b,
            ),
            (
                format!("{TAKEOUT_ROOT}/{TAKE_C}"),
                photo_c,
            ),
        ],
    )?;

    let contents = fs::read(&staging_zip).await?;
    upload_file(context, ZIP_1_NAME, &contents).await?;

    wait_import_done(context, BATCH_1, 1).await?;

    // Albums: title/description from metadata.json, batch id recorded.
    let italie = takeout_album(context, "Get-Italië 2016").await?;
    assert_eq!(italie.takeout_id.as_deref(), Some(BATCH_1));
    assert_eq!(italie.description.as_deref(), Some("Takeout test album"));

    let photos2024 = takeout_album(context, "Photos from 2024").await?;
    assert_eq!(photos2024.takeout_id.as_deref(), Some(BATCH_1));

    let albums = all_takeout_albums(context).await?;
    assert_eq!(
        albums,
        vec![
            ("Get-Italië 2016".to_owned(), BATCH_1.to_owned()),
            ("Photos from 2024".to_owned(), BATCH_1.to_owned()),
        ]
    );

    // Media items: canonical copies only; no Google Photos component and no
    // batch id in the path; sidecars must not have become items.
    let relative_a = format!("{prefix}takeout/Italië 2016/{TAKE_A}");
    let relative_b = format!("{prefix}takeout/Photos from 2024/{TAKE_B}");
    let relative_c = format!("{prefix}takeout/{TAKE_C}");
    wait_item_ingested(context, &relative_a).await?;
    wait_item_ingested(context, &relative_b).await?;
    wait_item_ingested(context, &relative_c).await?;

    let mut expected_paths = vec![relative_a.clone(), relative_b, relative_c];
    expected_paths.sort();
    let actual_paths = takeout_media_paths(context).await?;
    assert_eq!(actual_paths, expected_paths);

    // Filenames claimed via the pending-link mechanism during registration end
    // up attached to their album when this ingest consumed them.
    let item_ids = media_item_ids_for_filename(context, TAKE_A).await?;
    assert_eq!(item_ids.len(), 1, "filename not deduped into a single item");
    assert_eq!(
        takeout_album_ids_for_item(context, &item_ids[0]).await?,
        vec![italie.id]
    );

    // The uploaded zip is gone after a successful import.
    assert!(!std::fs::exists(
        context
            .settings
            .ingest
            .app_data_root
            .join("takeout")
            .join(BATCH_1)
            .join(ZIP_1_NAME)
    )?);

    info!("test_takeout_import OK");
    Ok(())
}

/// Filename dedup against existing library items, the fallback batch id for
/// non-standard zip names, and one filename shared between two albums.
pub async fn test_takeout_dedup_and_fallback_batch(context: &TestContext) -> Result<()> {
    login(context).await?;
    let user = user_media_folder(context).await?;
    let prefix = path_prefix(&user.1);

    let photo_a = read_source_photo(context, SOURCE_PHOTO_A).await?;
    let photo_b = read_source_photo(context, SOURCE_PHOTO_B).await?;

    // The zip filename does not follow the Google timestamp pattern, so the
    // whole batch gets the fallback id `takeout`.
    let staging = context
        .settings
        .ingest
        .app_data_root
        .join("takeout")
        .join("takeout");
    std::fs::create_dir_all(&staging)?;
    let staging_zip = staging.join(ZIP_RENAMED_NAME);
    write_zip(
        &staging_zip,
        &[
            (
                format!("{TAKEOUT_ROOT}/Second/metadata.json"),
                br#"{"title":"Second Album"}"#.to_vec(),
            ),
            // Already in the library (imported by the first test): no new item,
            // just a consumed link + duplicate file deletion.
            (
                format!("{TAKEOUT_ROOT}/Second/{SOURCE_PHOTO_A}"),
                photo_a.clone(),
            ),
            // Canonical copy for the fallback batch.
            (
                format!("{TAKEOUT_ROOT}/Second/{TESTIMG_A}"),
                photo_b.clone(),
            ),
            // Same filename in a second album: shares the item of Second.
            (format!("{TAKEOUT_ROOT}/Third/{TESTIMG_A}"), photo_b.clone()),
        ],
    )?;

    let contents = fs::read(&staging_zip).await?;
    upload_file(context, ZIP_RENAMED_NAME, &contents).await?;

    wait_import_done(context, "takeout", 1).await?;

    let second = takeout_album(context, "Second Album").await?;
    assert_eq!(second.takeout_id.as_deref(), Some("takeout"));

    wait_item_ingested(context, &format!("{prefix}takeout/Second/{TESTIMG_A}")).await?;

    // The library filename was NOT imported again: no item lives at the Second
    // folder path, and the consumed link points at an already-existing item.
    let second_folder_path = format!("{prefix}takeout/Second/{SOURCE_PHOTO_A}");
    let path_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM media_item WHERE relative_path = $1",
        second_folder_path
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(path_count, Some(0), "library filename was imported again");
    let consumed_id: Option<String> = sqlx::query_scalar!(
        r#"
        SELECT media_item_id
        FROM takeout_album_link
        WHERE album_id = $1 AND filename = $2 AND media_item_id IS NOT NULL
        "#,
        second.id,
        SOURCE_PHOTO_A
    )
    .fetch_optional(&context.pool)
    .await?
    .flatten();
    assert!(
        consumed_id.is_some(),
        "library filename has no consumed link in Second Album"
    );

    // The in-zip duplicate of TESTIMG_A shares one item between Second and Third.
    wait_item_ingested(context, &format!("{prefix}takeout/Second/{TESTIMG_A}")).await?;
    let testimg_ids = media_item_ids_for_filename(context, TESTIMG_A).await?;
    assert_eq!(testimg_ids.len(), 1, "in-zip duplicate was imported twice");
    let albums_for_testimg = takeout_album_ids_for_item(context, &testimg_ids[0]).await?;
    assert_eq!(
        albums_for_testimg.len(),
        2,
        "shared item not in both albums"
    );

    // The redundant on-disk copy for the already-library filename was removed.
    assert!(!std::fs::exists(
        context
            .settings
            .ingest
            .media_root
            .join(&prefix)
            .join("takeout")
            .join("Second")
            .join(SOURCE_PHOTO_A)
    )?);

    // Re-import the very same zip again: fully idempotent, no new items/albums.
    upload_file(context, ZIP_RENAMED_NAME, &contents).await?;
    wait_import_done(context, "takeout", 2).await?;
    let testimg_after = media_item_ids_for_filename(context, TESTIMG_A).await?;
    assert_eq!(testimg_after.len(), 1, "re-import created a duplicate item");
    let second_after = takeout_album(context, "Second Album").await?;
    assert_eq!(
        second_after.id, second.id,
        "re-import created another album"
    );

    info!("test_takeout_dedup_and_fallback_batch OK");
    Ok(())
}

/// Two chunked zips of one export share a single album via the parsed batch id,
/// and a filename from another batch dedups instead of importing again.
pub async fn test_takeout_chunked_batches(context: &TestContext) -> Result<()> {
    login(context).await?;
    let user = user_media_folder(context).await?;
    let prefix = path_prefix(&user.1);

    let photo_a = read_source_photo(context, SOURCE_PHOTO_A).await?;
    let photo_b = read_source_photo(context, SOURCE_PHOTO_B).await?;
    let photo_c = read_source_photo(context, SOURCE_PHOTO_C).await?;

    let chunk_dir = context
        .settings
        .ingest
        .app_data_root
        .join("takeout")
        .join(BATCH_2);
    std::fs::create_dir_all(&chunk_dir)?;
    write_zip(
        &chunk_dir.join("takeout-20261016T000000Z-1-001.zip"),
        &[(
            format!("{TAKEOUT_ROOT}/Photos from 2025/{TESTIMG_CHUNK_1}"),
            photo_a,
        )],
    )?;
    write_zip(
        &chunk_dir.join("takeout-20261016T000000Z-1-002.zip"),
        &[
            (
                format!("{TAKEOUT_ROOT}/Photos from 2025/{TESTIMG_CHUNK_2}"),
                photo_c,
            ),
            // TESTIMG_A was imported by the fallback batch already: cross-batch
            // filename dedup must keep the library item and just attach the
            // album.
            (
                format!("{TAKEOUT_ROOT}/Photos from 2025/{TESTIMG_A}"),
                photo_b,
            ),
        ],
    )?;

    let zip_a = fs::read(chunk_dir.join("takeout-20261016T000000Z-1-001.zip")).await?;
    upload_file(context, "takeout-20261016T000000Z-1-001.zip", &zip_a).await?;
    wait_import_done(context, BATCH_2, 1).await?;
    wait_item_ingested(
        context,
        &format!("{prefix}takeout/Photos from 2025/{TESTIMG_CHUNK_1}"),
    )
    .await?;

    let zip_b = fs::read(chunk_dir.join("takeout-20261016T000000Z-1-002.zip")).await?;
    upload_file(context, "takeout-20261016T000000Z-1-002.zip", &zip_b).await?;
    wait_import_done(context, BATCH_2, 2).await?;
    wait_item_ingested(
        context,
        &format!("{prefix}takeout/Photos from 2025/{TESTIMG_CHUNK_2}"),
    )
    .await?;

    // One album created by the two chunks; both chunk files are its members.
    let photos2025 = takeout_album(context, "Photos from 2025").await?;
    assert_eq!(photos2025.takeout_id.as_deref(), Some(BATCH_2));
    let chunk1_ids = media_item_ids_for_filename(context, TESTIMG_CHUNK_1).await?;
    let chunk2_ids = media_item_ids_for_filename(context, TESTIMG_CHUNK_2).await?;
    assert_eq!(chunk1_ids.len(), 1);
    assert_eq!(chunk2_ids.len(), 1);
    let photo2025_id = photos2025.id.clone();
    assert_eq!(
        takeout_album_ids_for_item(context, &chunk1_ids[0]).await?,
        vec![photo2025_id.clone()]
    );
    assert_eq!(
        takeout_album_ids_for_item(context, &chunk2_ids[0]).await?,
        vec![photo2025_id]
    );

    // The cross-batch file did not create a second item either.
    let testimg_ids = media_item_ids_for_filename(context, TESTIMG_A).await?;
    assert_eq!(testimg_ids.len(), 1, "cross-batch dedup failed");

    info!("test_takeout_chunked_batches OK");
    Ok(())
}
