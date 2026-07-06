use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use app_state::constants::ALBUM_IMPORT_FOLDER;
use color_eyre::eyre::{Result, bail};
use common_services::api::album::interfaces::{
    AcceptInviteRequest, AddMediaToAlbumRequest, CheckInviteRequest, CreateAlbumRequest,
    UpdateAlbumRequest,
};
use common_services::database::UpdateField;
use common_services::database::album::album::{Album, AlbumSummary};
use common_services::database::album_store::AlbumStore;
use common_services::database::user_store::UserStore;
use common_types::dev_constants::{EMAIL, USERNAME};
use reqwest::StatusCode;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::time::sleep;
use tracing::info;

pub async fn test_album_lifecycle(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;

    // 1. Create Album
    let create_payload = CreateAlbumRequest {
        name: "Lifecycle Test Album".to_string(),
        description: Some("Integration test description".to_string()),
        is_public: false,
        media_item_ids: vec![],
    };

    let response = client
        .post(format!("{base_url}/album"))
        .bearer_auth(&token)
        .json(&create_payload)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let created_album: Album = response.json().await?;
    assert_eq!(created_album.name, create_payload.name);

    // 2. List Albums
    let response = client
        .get(format!("{base_url}/album"))
        .bearer_auth(&token)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let albums: Vec<Album> = response.json().await?;
    assert!(albums.iter().any(|a| a.id == created_album.id));

    // 3. Get Album Details
    // todo: get album details via /ratios, /ids, /by-month

    Ok(())
}

pub async fn test_update_album(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;

    // Create an album first
    let created_album: Album = client
        .post(format!("{base_url}/album"))
        .bearer_auth(&token)
        .json(&CreateAlbumRequest {
            name: "Original Name".to_string(),
            description: None,
            is_public: false,
            media_item_ids: vec![],
        })
        .send()
        .await?
        .json()
        .await?;

    // ACT - Update Name and Description
    let update_payload = UpdateAlbumRequest {
        name: Some("Updated Name".to_string()),
        description: UpdateField::Value("Updated Description".to_string()),
        is_public: Some(true),
        ..Default::default()
    };

    let response = client
        .put(format!("{base_url}/album/{}", created_album.id))
        .bearer_auth(&token)
        .json(&update_payload)
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let updated_album: Album = response.json().await?;

    assert_eq!(updated_album.name, "Updated Name");
    assert_eq!(
        updated_album.description.as_deref(),
        Some("Updated Description")
    );
    assert!(updated_album.is_public);

    Ok(())
}

pub async fn test_album_media_management(context: &TestContext) -> Result<()> {
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;

    // Find a valid media item ID from DB
    let media_item = sqlx::query!("SELECT id FROM media_item LIMIT 1")
        .fetch_optional(&context.pool)
        .await?;

    let Some(media_record) = media_item else {
        bail!("Skipping album media test: No media items found in DB. Run onboarding tests first.");
    };
    let media_id = media_record.id;

    // Create Album
    let album: Album = client
        .post(format!("{base_url}/album"))
        .bearer_auth(&token)
        .json(&CreateAlbumRequest {
            name: "Media Test Album".to_string(),
            description: None,
            is_public: false,
            media_item_ids: vec![],
        })
        .send()
        .await?
        .json()
        .await?;

    // 1. Add Media to Album
    let response = client
        .post(format!("{base_url}/album/{}/media", album.id))
        .bearer_auth(&token)
        .json(&AddMediaToAlbumRequest {
            media_item_ids: vec![media_id.clone()],
        })
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    // Verify addition via details
    // todo verify with /ids, /ratios, /by-month, maybe just put back the get album details endpoint

    // 2. Remove Media from Album
    let response = client
        .delete(format!("{base_url}/album/{}/media/{}", album.id, media_id))
        .bearer_auth(&token)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify removal
    // todo verify with /ids, /ratios, /by-month, maybe just put back the get album details endpoint

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn test_album_sharing(context: &TestContext) -> Result<()> {
    // # Generate JWT invite
    // ARRANGE
    let token = login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;
    let source_album_name = "Invite Test Album";
    // Find a valid media item ID from DB
    let Some(media_id) = sqlx::query_scalar!("SELECT id FROM media_item LIMIT 1")
        .fetch_optional(&context.pool)
        .await?
    else {
        bail!("Skipping album media test: No media items found in DB. Run onboarding tests first.");
    };

    let album: Album = client
        .post(format!("{base_url}/album"))
        .bearer_auth(&token)
        .json(&CreateAlbumRequest {
            name: source_album_name.to_string(),
            description: None,
            is_public: false,
            media_item_ids: vec![],
        })
        .send()
        .await?
        .json()
        .await?;
    // Add Media to Album
    let response = client
        .post(format!("{base_url}/album/{}/media", album.id))
        .bearer_auth(&token)
        .json(&AddMediaToAlbumRequest {
            media_item_ids: vec![media_id.clone()],
        })
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // ACT
    let response = client
        .get(format!("{base_url}/album/{}/invite", album.id))
        .bearer_auth(&token)
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let invite_token: String = response.json().await?;

    // # Check JWT invite
    let response = client
        .post(format!("{base_url}/album/invite/check"))
        .bearer_auth(&token)
        .json(&CheckInviteRequest {
            token: invite_token.clone(),
        })
        .send()
        .await?;
    let status_code = response.status();
    assert_eq!(status_code, StatusCode::OK);
    let album_summary: AlbumSummary = response.json().await?;
    assert_eq!(album_summary.name, source_album_name);
    assert_eq!(album_summary.description, None);
    for rel_path in album_summary.relative_paths {
        assert!(context.settings.ingest.media_root.join(&rel_path).exists());
    }

    // # Accept JWT invite
    let imported_album_name = "My new imported album";
    let imported_album_description = "New description";
    let response = client
        .post(format!("{base_url}/album/invite/accept"))
        .bearer_auth(&token)
        .json(&AcceptInviteRequest {
            token: invite_token.clone(),
            name: imported_album_name.to_owned(),
            description: Some(imported_album_description.to_owned()),
        })
        .send()
        .await?;
    let status_code = response.status();
    assert_eq!(status_code, StatusCode::OK);
    let album: Album = response.json().await?;
    assert_eq!(album.name, imported_album_name);
    assert_eq!(
        album.description,
        Some(imported_album_description.to_owned())
    );

    // Wait for imported items to be processed
    let start_time = Instant::now();
    let timeout = Duration::from_mins(1);
    info!("Waiting for album import to complete.");
    // todo: om een of andere reden wordt de import album item job niet opgepakt?
    loop {
        let album_content = AlbumStore::list_media_items(&context.pool, &album.id).await?;
        if !album_content.is_empty() {
            break;
        }
        if start_time.elapsed() > timeout {
            bail!("Timeout waiting for import album items to show up.");
        }
        sleep(Duration::from_secs(2)).await;
    }

    let album_content = AlbumStore::list_media_items(&context.pool, &album.id).await?;
    assert_eq!(album_content.len(), 1);
    let album_item = album_content.first().expect("There should be an item here");
    let new_media_id = album_item.media_item.id.clone();
    let Some(remote_user_id) = sqlx::query_scalar!(
        "SELECT remote_user_id FROM media_item WHERE id = $1",
        new_media_id
    )
    .fetch_one(&context.pool)
    .await?
    else {
        bail!("Remote user id set on imported media item");
    };
    {
        struct RemoteUser {
            user_id: i32,
            identity: String,
            name: Option<String>,
        }
        let remote_user = sqlx::query_as!(
            RemoteUser,
            "SELECT identity, name, user_id FROM remote_user WHERE id = $1",
            remote_user_id
        )
        .fetch_one(&context.pool)
        .await?;
        let owning_user = UserStore::find_by_id(&context.pool, remote_user.user_id)
            .await?
            .expect("This user should exist.");
        assert_eq!(remote_user.name, None);
        assert_eq!(remote_user.user_id, owning_user.id);
        assert!(remote_user.identity.contains(&format!("{USERNAME}@")));

        assert_eq!(owning_user.email, EMAIL);
        assert_eq!(owning_user.name, USERNAME);
    }

    // Check for import folder structure
    let import_folder = context.settings.ingest.media_root.join(ALBUM_IMPORT_FOLDER);
    assert!(import_folder.exists());
    let dirs = {
        let mut dirs = Vec::new();
        let mut rd = fs::read_dir(import_folder).await?;
        while let Some(entry) = rd.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                dirs.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        dirs
    };
    assert_eq!(dirs.len(), 1);

    Ok(())
}
