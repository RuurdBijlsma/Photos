use crate::runner::context::test_context::TestContext;
use crate::test_helpers::{login, media_dir_contents};
use app_state::MakeRelativePath;
use color_eyre::eyre::{ContextCompat, Result, bail};
use common_services::api::album::interfaces::{
    AddMediaToAlbumRequest, CreateAlbumRequest, MediaItemWithAlbums,
};
use common_services::api::theme::interfaces::RandomPhotoResponse;
use common_services::database::album::album::Album;
use material_color_utils::dynamic::variant::Variant;
use reqwest::StatusCode;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::time::sleep;
use tracing::info;

pub async fn test_photo_download(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let (photos, videos) = media_dir_contents(context)?;
    let client = &context.http_client;
    let url = format!("{}/photos/download", context.settings.api.public_url);

    // --- TEST 1: Download a Photo (Happy Path) ---
    if let Some(photo_path) = photos.first() {
        let relative_path = photo_path.make_relative(&context.settings.ingest.media_root)?;

        let response = client
            .get(&url)
            .query(&[("path", &relative_path)])
            .bearer_auth(&token)
            .send()
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        // Check Headers
        let content_type = response
            .headers()
            .get("content-type")
            .expect("content type header")
            .to_str()?;
        assert!(
            content_type.starts_with("image/"),
            "Expected image mime type, got {content_type}"
        );

        // Check Body Size matches File Size
        let file_size = fs::metadata(photo_path).await?.len();
        let bytes = response.bytes().await?;
        assert_eq!(
            bytes.len() as u64,
            file_size,
            "Downloaded byte count mismatch for photo"
        );
    } else {
        println!("Skipping photo download test (no photos found in assets)");
    }

    // --- TEST 2: Download a Video (Happy Path) ---
    if let Some(video_path) = videos.first() {
        let relative_path = video_path.make_relative(&context.settings.ingest.media_root)?;

        let response = client
            .get(&url)
            .query(&[("path", &relative_path)])
            .bearer_auth(&token)
            .send()
            .await?;

        assert_eq!(response.status(), StatusCode::OK);

        let file_size = fs::metadata(video_path).await?.len();
        let bytes = response.bytes().await?;
        assert_eq!(
            bytes.len() as u64,
            file_size,
            "Downloaded byte count mismatch for video"
        );
    } else {
        println!("Skipping video download test (no videos found in assets)");
    }

    // --- TEST 3: File Not Found ---
    let response = client
        .get(&url)
        .query(&[("path", "non_existent_file.jpg")])
        .bearer_auth(&token)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // --- TEST 4: Path Traversal Attack (Security) ---
    let media_root = &context.settings.ingest.media_root;
    let parent_dir = media_root.parent().context("Media root has no parent")?;
    let secret_file = parent_dir.join("secret_system_file.txt");
    fs::write(&secret_file, "super secret content").await?;

    let traversal_path = "../secret_system_file.txt";

    let response = client
        .get(&url)
        .query(&[("path", traversal_path)])
        .bearer_auth(&token)
        .send()
        .await?;

    // Cleanup
    let _ = fs::remove_file(secret_file).await;

    // The service checks `!file_canon.starts_with(&media_dir_canon)` -> returns InvalidPath (400)
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Traversal attempt should return 400 Bad Request"
    );

    // --- TEST 5: Unauthorized Access ---
    if let Some(photo_path) = photos.first() {
        let relative_path = photo_path.make_relative(&context.settings.ingest.media_root)?;

        let response = client
            .get(&url)
            .query(&[("path", &relative_path)])
            // No Bearer Auth
            .send()
            .await?;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    Ok(())
}

pub async fn test_get_full_item(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;

    // Get a valid ID from the database
    let media_item = sqlx::query!("SELECT id FROM media_item LIMIT 1")
        .fetch_optional(&context.pool)
        .await?;

    if let Some(record) = media_item {
        let valid_id = record.id;
        let valid_id_url = format!("{}/photos/{valid_id}/item", context.settings.api.public_url);

        // --- TEST 1: Valid ID ---
        let response = client.get(&valid_id_url).bearer_auth(&token).send().await?;

        assert_eq!(response.status(), StatusCode::OK);
        let item: MediaItemWithAlbums = response.json().await?;
        assert_eq!(item.media_item.id, valid_id);

        // --- TEST 2: Invalid ID ---
        let invalid_id_url = format!("{}/photos/invalid-id/item", context.settings.api.public_url);
        let response = client
            .get(&invalid_id_url)
            .bearer_auth(&token)
            .send()
            .await?;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    } else {
        // This might happen if previous tests failed to populate DB
        println!("Skipping test_get_full_item (no media items in DB)");
    }

    Ok(())
}

pub async fn test_get_full_item_albums(context: &TestContext) -> Result<()> {
    let token = login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;

    let Some(media_record) = sqlx::query!("SELECT id FROM media_item LIMIT 1")
        .fetch_optional(&context.pool)
        .await?
    else {
        println!("Skipping test_get_full_item_albums (no media items in DB)");
        return Ok(());
    };
    let media_id = media_record.id;

    let album: Album = client
        .post(format!("{base_url}/album"))
        .bearer_auth(&token)
        .json(&CreateAlbumRequest {
            name: "Info Panel Album".to_string(),
            description: None,
            is_public: false,
            media_item_ids: vec![],
        })
        .send()
        .await?
        .json()
        .await?;

    let response = client
        .post(format!("{base_url}/album/{}/media", album.id))
        .bearer_auth(&token)
        .json(&AddMediaToAlbumRequest {
            media_item_ids: vec![media_id.clone()],
        })
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let item_url = format!("{base_url}/photos/{media_id}/item");
    let response = client.get(&item_url).bearer_auth(&token).send().await?;
    assert_eq!(response.status(), StatusCode::OK);

    let item: MediaItemWithAlbums = response.json().await?;
    assert_eq!(item.media_item.id, media_id);
    let fetched_album = item
        .albums
        .iter()
        .find(|a| a.id == album.id)
        .expect("Album not found in media item's albums");
    assert_eq!(fetched_album.name, "Info Panel Album");
    assert_eq!(fetched_album.media_count, 1);

    Ok(())
}

pub async fn test_get_color_theme(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let url = format!("{}/theme/by-color", context.settings.api.public_url);
    let color = "#FF5733";
    let theme_variant =
        serde_json::to_value(context.settings.ingest.analyzer.theme_generation.variant)?
            .as_str()
            .context("theme variant should serialize to a string")?
            .to_string();

    // ACT
    let response = client
        .get(&url)
        .query(&[
            ("color", color),
            ("variant", theme_variant.as_str()),
            ("contrast", "0.2"),
        ])
        .bearer_auth(&token)
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await?;

    // Verify it looks like a theme object
    assert!(body.is_object());
    if let Some(obj) = body.as_object() {
        assert!(obj.contains_key("source_color"));
        assert!(obj.contains_key("schemes"));
        let variant = obj
            .get("variant")
            .and_then(|v| v.as_str())
            .expect("key: variant not found");
        let variant_from_json: Variant = serde_json::from_str(&format!("\"{variant}\""))?;
        assert_eq!(
            variant_from_json,
            context.settings.ingest.analyzer.theme_generation.variant
        );
    }

    Ok(())
}

pub async fn test_get_random_photo(context: &TestContext) -> Result<()> {
    let timeout = Duration::from_mins(2);
    let start = Instant::now();
    info!("Waiting for 1 analysis job to complete...");
    loop {
        // First we need to wait for at least 1 analysis job to complete
        let ids = sqlx::query_scalar!("SELECT visual_analysis_id FROM color")
            .fetch_all(&context.pool)
            .await?;
        if !ids.is_empty() {
            break;
        }
        if start.elapsed() > timeout {
            bail!("Timed out while waiting for 1 analysis job to complete");
        }
        sleep(Duration::from_secs(2)).await;
    }

    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let url = format!("{}/theme/random-photo", context.settings.api.public_url);
    let theme_variant =
        serde_json::to_value(context.settings.ingest.analyzer.theme_generation.variant)?
            .as_str()
            .context("theme variant should serialize to a string")?
            .to_string();

    // ACT
    let response = client
        .get(&url)
        .query(&[("variant", theme_variant.as_str()), ("contrast", "0.2")])
        .bearer_auth(&token)
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let body: Option<RandomPhotoResponse> = response.json().await?;

    let Some(data) = body else {
        bail!("No random photo data found");
    };
    assert!(!data.media_id.is_empty());
    assert_eq!(
        data.theme.variant,
        context.settings.ingest.analyzer.theme_generation.variant
    );

    Ok(())
}
