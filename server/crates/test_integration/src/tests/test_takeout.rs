use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use color_eyre::Result;
use color_eyre::eyre::bail;
use common_services::database::album::album::Album;
use common_services::database::jobs::JobType;
use common_services::job_queue::enqueue_job;
use reqwest::StatusCode;
use std::io::Write;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use zip::write::SimpleFileOptions;

pub async fn test_takeout_import_flow(context: &TestContext) -> Result<()> {
    // 1. Authenticate user
    login(context).await?;
    let api_url = &context.settings.api.public_url;
    let client = &context.http_client;

    let user_id = sqlx::query_scalar!("SELECT id FROM app_user LIMIT 1")
        .fetch_one(&context.pool)
        .await?;

    // 2. Prepare synthetic Takeout ZIP archives
    let app_data_tus = context
        .settings
        .ingest
        .app_data_root
        .join(app_state::constants::TUS_UPLOADS_FOLDER);
    tokio::fs::create_dir_all(&app_data_tus).await?;

    let batch_id = "test_takeout_batch_123".to_string();
    let zip1_id = format!("{batch_id}_part1");
    let zip2_id = format!("{batch_id}_part2");

    let zip1_path = app_data_tus.join(&zip1_id);
    let zip2_path = app_data_tus.join(&zip2_id);

    // ZIP Part 1: Contains an album folder with a photo and album metadata.json
    {
        let file = std::fs::File::create(&zip1_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        zip.start_file(
            "Takeout/Google Photos/Vacation 2023 /metadata.json",
            options,
        )?;
        zip.write_all(br#"{"title": "Vacation 2023", "description": "Summer holiday"}"#)?;

        // Include a sample 1x1 test image
        zip.start_file(
            "Takeout/Google Photos/Vacation 2023 /takeout_photo_1.jpg",
            options,
        )?;
        let sample_media = context
            .settings
            .ingest
            .media_root
            .join("Ruurd")
            .join("PXL_20250916_155329360.jpg");
        if let Ok(bytes) = std::fs::read(&sample_media) {
            zip.write_all(&bytes)?;
        } else {
            zip.write_all(b"fake jpeg content")?;
        }
        zip.finish()?;
    }

    // ZIP Part 2: Contains sidecar metadata for the photo from part 1, plus a year album folder
    {
        let file = std::fs::File::create(&zip2_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        zip.start_file(
            "Takeout/Google Photos/Photos from 2023/takeout_photo_1.jpg.supplemental-metadata.json",
            options,
        )?;
        zip.write_all(
            br#"{
            "title": "takeout_photo_1.jpg",
            "photoTakenTime": {
                "timestamp": "1690000000"
            }
        }"#,
        )?;

        zip.finish()?;
    }

    // 3. Enqueue IngestTakeoutBatch directly as if all TUS uploads finished
    #[derive(serde::Serialize)]
    struct TakeoutBatchPayload {
        batch_id: String,
        tus_upload_ids: Vec<String>,
    }

    let payload = TakeoutBatchPayload {
        batch_id: batch_id.clone(),
        tus_upload_ids: vec![zip1_id.clone(), zip2_id.clone()],
    };

    enqueue_job(
        &context.pool,
        &context.settings.ingest,
        JobType::IngestTakeoutBatch,
    )
    .user_id(user_id)
    .payload(&payload)
    .call()
    .await?;

    // 4. Poll database until album "Vacation 2023" exists
    let timeout = Duration::from_secs(45);
    let start = Instant::now();
    let mut album_found = false;

    while start.elapsed() < timeout {
        let exists = sqlx::query_scalar!(
            "SELECT COUNT(id) FROM album WHERE owner_id = $1 AND name = $2",
            user_id,
            "Vacation 2023"
        )
        .fetch_one(&context.pool)
        .await?
        .unwrap_or(0);

        if exists > 0 {
            album_found = true;
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    if !album_found {
        bail!("IngestTakeoutBatch failed to create 'Vacation 2023' album within timeout");
    }

    // 5. Verify the album details through the REST API
    let resp = client.get(format!("{api_url}/api/album")).send().await?;
    assert_eq!(resp.status(), StatusCode::OK);
    let albums: Vec<Album> = resp.json().await?;
    let vacation_album = albums.iter().find(|a| a.name == "Vacation 2023");
    assert!(vacation_album.is_some());
    assert_eq!(
        vacation_album.unwrap().description.as_deref(),
        Some("Summer holiday")
    );

    // 6. Verify that takeout files and temporary directories were cleaned up
    assert!(!zip1_path.exists());
    assert!(!zip2_path.exists());

    Ok(())
}
