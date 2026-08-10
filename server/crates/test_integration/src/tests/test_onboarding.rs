use crate::runner::context::test_context::TestContext;
use crate::test_helpers::{login, media_dir_contents};
use app_state::MakeRelativePath;
use color_eyre::Result;
use color_eyre::eyre::bail;
use common_services::api::admin::interfaces::{
    DiskResponse, MakeFolderBody, MediaSampleResponse, UnsupportedFilesResponse,
    UpdateUserMediaFolderBody,
};
use reqwest::StatusCode;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::time::sleep;
use tracing::info;

pub async fn test_onboarding(context: &TestContext) -> Result<()> {
    // 1. Login
    login(context).await?;
    let api_url = &context.settings.api.public_url;
    let client = &context.http_client;

    // 2. Get Disk Info
    let response = client
        .get(format!("{api_url}/api/admin/disk-info"))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let disk_info: DiskResponse = response.json().await?;
    assert!(disk_info.media_folder.read_access);
    assert!(disk_info.app_data_folder.read_access);

    // 3. List Folders (root)
    let response = client
        .get(format!("{api_url}/api/admin/folders"))
        .query(&[("folder", "")])
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let _folders: Vec<String> = response.json().await?;

    // 4. Create a new folder
    let created_folder = "integration_test_folder";
    let response = client
        .post(format!("{api_url}/api/admin/make-folder"))
        .json(&MakeFolderBody {
            base_folder: String::new(),
            new_name: created_folder.to_string(),
        })
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 5. Verify the folder exists
    let response = client
        .get(format!("{api_url}/api/admin/folders"))
        .query(&[("folder", "")])
        .send()
        .await?;

    let folders: Vec<String> = response.json().await?;
    assert!(folders.contains(&created_folder.to_string()));
    let folder = &context.settings.ingest.media_root.join(created_folder);
    assert!(folder.exists());
    fs::remove_dir(folder).await?;

    // 6. Check Media Sample
    let response = client
        .get(format!("{api_url}/api/admin/media-sample"))
        .query(&[("folder", "")])
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let sample: MediaSampleResponse = response.json().await?;
    let (photos, videos) = media_dir_contents(context)?;
    assert_eq!(sample.photo_count, photos.len());
    assert_eq!(sample.video_count, videos.len());

    // 7. Check Unsupported Files
    let response = client
        .get(format!("{api_url}/api/admin/unsupported-files"))
        .query(&[("folder", "")])
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let unsupported: UnsupportedFilesResponse = response.json().await?;
    assert_eq!(unsupported.unsupported_count, 0);

    Ok(())
}

pub async fn test_start_processing(context: &TestContext) -> Result<()> {
    // 1. Login
    login(context).await?;
    let api_url = &context.settings.api.public_url;
    let client = &context.http_client;

    // 2. Prepare expected counts
    let (photos, videos) = media_dir_contents(context)?;
    let expected_media_items = photos.len() + videos.len();

    // 3. Start Processing
    let user_id = 1;
    let response = client
        .put(format!("{api_url}/api/admin/users/{user_id}/media-folder"))
        .json(&UpdateUserMediaFolderBody {
            user_folder: String::new(),
        })
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 4. Poll database until all media items are inserted
    let timeout = Duration::from_mins(1);
    let start = Instant::now();
    info!(
        "Waiting for {} media items to be inserted into DB...",
        expected_media_items
    );

    loop {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(id) FROM media_item WHERE user_id = $1",
            user_id
        )
        .fetch_one(&context.pool)
        .await?
        .unwrap_or(0);

        if count == expected_media_items as i64 {
            break;
        }

        if start.elapsed() > timeout {
            bail!(
                "Processing media files took longer than the timeout: {:?}. Inserted {}/{}",
                timeout,
                count,
                expected_media_items
            );
        }

        sleep(Duration::from_millis(500)).await;
    }
    info!("All media items are processed");

    // 5. Check if media item relative paths match actual files in media root.
    let db_paths: HashSet<String> = sqlx::query_scalar!("SELECT relative_path FROM media_item")
        .fetch_all(&context.pool)
        .await?
        .into_iter()
        .collect();
    let fs_paths: HashSet<_> = photos
        .into_iter()
        .chain(videos)
        .map(|p| p.make_relative(&context.settings.ingest.media_root))
        .collect::<Result<_>>()?;
    assert_eq!(db_paths, fs_paths);

    // 6. Wait for ingest thumbnails to complete
    let start = Instant::now();
    let timeout = Duration::from_mins(2);
    loop {
        let jobs_completed = sqlx::query_scalar!(
            "SELECT COUNT(id) FROM jobs WHERE job_type = 'ingest_thumbnails' AND status = 'done'"
        )
        .fetch_one(&context.pool)
        .await?
        .expect("Couldn't get ingest_thumbnails job count");
        if jobs_completed == expected_media_items as i64 {
            break;
        }
        if start.elapsed() > timeout {
            bail!("Timed out while waiting for thumbnail jobs to complete");
        }
        sleep(Duration::from_secs(2)).await;
    }

    // 7. Check if thumbnails exist on disk.
    {
        struct MediaItem {
            id: String,
            relative_path: String,
        }
        let media_items = sqlx::query_as!(MediaItem, "SELECT id, relative_path FROM media_item")
            .fetch_all(&context.pool)
            .await?;
        for item in &media_items {
            let path = context.settings.ingest.media_root.join(&item.relative_path);
            let thumbs_exist = context
                .settings
                .ingest
                .thumbs_exist(&path, &item.id, None, false)?;
            assert!(thumbs_exist);
        }
    }

    Ok(())
}
