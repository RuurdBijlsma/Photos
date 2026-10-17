use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use app_state::constants::ALBUM_IMPORT_FOLDER;
use color_eyre::eyre::{Result, bail};
use common_services::api::album::interfaces::{
    AcceptInviteRequest, AddMediaToAlbumRequest, AlbumSort, CheckInviteRequest, CreateAlbumRequest,
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
    login(context).await?;
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
        .post(format!("{base_url}/api/album"))
        .json(&create_payload)
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let created_album: Album = response.json().await?;
    assert_eq!(created_album.name, create_payload.name);

    // 2. List Albums
    let response = client.get(format!("{base_url}/api/album")).send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let albums: Vec<Album> = response.json().await?;
    assert!(albums.iter().any(|a| a.id == created_album.id));

    Ok(())
}

pub async fn test_update_album(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;

    // Create an album first
    let created_album: Album = client
        .post(format!("{base_url}/api/album"))
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
        .put(format!("{base_url}/api/album/{}", created_album.id))
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
    login(context).await?;
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
        .post(format!("{base_url}/api/album"))
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
        .post(format!("{base_url}/api/album/{}/media", album.id))
        .json(&AddMediaToAlbumRequest {
            media_item_ids: vec![media_id.clone()],
        })
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    // 2. Remove Media from Album
    let response = client
        .delete(format!(
            "{base_url}/api/album/{}/media/{}",
            album.id, media_id
        ))
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn test_album_sharing(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;
    let source_album_name = "Invite Test Album";

    let Some(media_id) = sqlx::query_scalar!("SELECT id FROM media_item LIMIT 1")
        .fetch_optional(&context.pool)
        .await?
    else {
        bail!("Skipping album media test: No media items found in DB. Run onboarding tests first.");
    };

    let album: Album = client
        .post(format!("{base_url}/api/album"))
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
        .post(format!("{base_url}/api/album/{}/media", album.id))
        .json(&AddMediaToAlbumRequest {
            media_item_ids: vec![media_id.clone()],
        })
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // ACT
    let response = client
        .get(format!("{base_url}/api/album/{}/invite", album.id))
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let invite_token: String = response.json().await?;

    // Check JWT invite
    let response = client
        .post(format!("{base_url}/api/album/invite/check"))
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

    // Accept JWT invite
    let imported_album_name = "My new imported album";
    let imported_album_description = "New description";
    let response = client
        .post(format!("{base_url}/api/album/invite/accept"))
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
    loop {
        let album_content =
            AlbumStore::list_media_items(&context.pool, &album.id, AlbumSort::None).await?;
        if !album_content.is_empty() {
            break;
        }
        if start_time.elapsed() > timeout {
            bail!("Timeout waiting for import album items to show up.");
        }
        sleep(Duration::from_secs(2)).await;
    }

    let album_content =
        AlbumStore::list_media_items(&context.pool, &album.id, AlbumSort::None).await?;
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

/// Rank is a manual-ordering override: populated iff `sort_mode = 'none'`.
/// Date-sorted albums derive their order from data columns; appends never
/// touch rank; reorder save materializes (or clears) ranks.
#[allow(clippy::too_many_lines)]
pub async fn test_album_rank_override(context: &TestContext) -> Result<()> {
    login(context).await?;
    let client = &context.http_client;
    let base_url = &context.settings.api.public_url;

    // Stable date ordering must exist for at least 4 items (after onboarding).
    let date_ids: Vec<String> = sqlx::query_scalar!(
        r#"
        SELECT id FROM media_item
        WHERE deleted = false
        ORDER BY sort_timestamp ASC, id ASC
        LIMIT 4
        "#,
    )
    .fetch_all(&context.pool)
    .await?;
    let [id_a, id_b, id_c, id_d] = date_ids.as_slice() else {
        bail!("Need at least 4 media items; run onboarding tests first.");
    };
    let rank_of = |album_id: &str, media_id: &str| {
        let pool = &context.pool;
        let album_id = album_id.to_owned();
        let media_id = media_id.to_owned();
        async move {
            sqlx::query_scalar!(
                "SELECT rank FROM album_media_item WHERE album_id = $1 AND media_item_id = $2",
                album_id,
                media_id
            )
            .fetch_one(pool)
            .await
            .expect("media item must be linked to the album")
        }
    };

    // 1. Fresh album (DateAsc default), add 3 items: appends must NOT write rank.
    let album: Album = client
        .post(format!("{base_url}/api/album"))
        .json(&serde_json::json!({
            "name": "Rank Override Test",
            "description": null,
            "isPublic": false,
            "mediaItemIds": []
        }))
        .send()
        .await?
        .json()
        .await?;

    for batch in [vec![id_a, id_b, id_c], vec![id_d]] {
        let response = client
            .post(format!("{base_url}/api/album/{}/media", album.id))
            .json(&serde_json::json!({ "mediaItemIds": batch.iter().collect::<Vec<_>>() }))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
    }

    for id in [&id_a, &id_b, &id_c, &id_d] {
        assert!(
            rank_of(&album.id, id).await.is_none(),
            "rank must stay NULL in a date-sorted album"
        );
    }
    let items =
        AlbumStore::list_sorted_media_items(&context.pool, &album.id, AlbumSort::DateAsc).await?;
    let date_order: Vec<String> = items.into_iter().map(|i| i.id).collect();
    assert_eq!(date_order.len(), 4);
    assert_date_sortedness(&context.pool, &date_order).await;

    // 2. Save a manual order: ranks materialize 1..N*1000 and the view follows.
    let response = client
        .put(format!("{base_url}/api/album/{}/media/reorder", album.id))
        .json(&serde_json::json!({
            "mediaItemIds": [&id_d, &id_c, &id_b, &id_a],
            "sortMode": "None",
        }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    for (position, id) in [&id_d, &id_c, &id_b, &id_a].iter().enumerate() {
        let rank = rank_of(&album.id, id)
            .await
            .expect("manual mode must materialize rank");
        let expected = ((position + 1) * 1000) as f64;
        assert_eq!(rank, expected, "rank of item {id}");
    }
    let items =
        AlbumStore::list_sorted_media_items(&context.pool, &album.id, AlbumSort::None).await?;
    let view_ids: Vec<String> = items.into_iter().map(|i| i.id).collect();
    assert_eq!(
        view_ids,
        vec![id_d.clone(), id_c.clone(), id_b.clone(), id_a.clone()],
        "manual view must exactly follow the saved order"
    );

    // 3. Appending into a manual album lands at the end (rank > MAX).
    let response = client
        .delete(format!("{base_url}/api/album/{}/media/{id_a}", album.id))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = client
        .post(format!("{base_url}/api/album/{}/media", album.id))
        .json(&serde_json::json!({ "mediaItemIds": [id_a] }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let appended_rank = rank_of(&album.id, id_a)
        .await
        .expect("manual album append gets rank");
    // The deleted item held the previous max (4000), so compare against the
    // remaining ranks: the new item must be strictly past all of them.
    for id in [&id_d, &id_c, &id_b] {
        let rank = rank_of(&album.id, id)
            .await
            .expect("remaining manual items keep their rank");
        assert!(
            appended_rank > rank,
            "appended rank must be past the remaining max ({appended_rank} vs {rank})"
        );
    }
    let items =
        AlbumStore::list_sorted_media_items(&context.pool, &album.id, AlbumSort::None).await?;
    let view_ids: Vec<String> = items.into_iter().map(|i| i.id).collect();
    assert_eq!(
        view_ids,
        vec![id_d.clone(), id_c.clone(), id_b.clone(), id_a.clone()],
        "re-appended item must land at the manual end"
    );

    // 4. Saving a data mode clears ranks again and the view re-derives from data.
    let response = client
        .put(format!("{base_url}/api/album/{}/media/reorder", album.id))
        .json(&serde_json::json!({
            "mediaItemIds": view_ids,
            "sortMode": "DateAsc",
        }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    for id in [&id_a, &id_b, &id_c, &id_d] {
        assert!(
            rank_of(&album.id, id).await.is_none(),
            "rank must be cleared when leaving manual mode"
        );
    }
    let items =
        AlbumStore::list_sorted_media_items(&context.pool, &album.id, AlbumSort::DateAsc).await?;
    let final_view: Vec<String> = items.into_iter().map(|i| i.id).collect();
    let mut expected = vec![id_a.clone(), id_b.clone(), id_c.clone(), id_d.clone()];
    expected.sort();
    let mut actual = final_view.clone();
    actual.sort();
    assert_eq!(
        actual, expected,
        "same items must come back after leaving manual mode"
    );
    assert_date_sortedness(&context.pool, &final_view).await;

    info!("Album rank override assertions passed.");
    Ok(())
}

/// Asserts that a date-asc view is non-decreasing by `(sort_timestamp,
/// created_at)`. Exact tie order is unspecified (deterministic tiebreaks
/// may differ), so only the semantic invariant is checked here.
async fn assert_date_sortedness(pool: &sqlx::PgPool, ids: &[String]) {
    use std::collections::HashMap;

    struct MetaRow {
        id: String,
        sort_timestamp: chrono::DateTime<chrono::Utc>,
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as!(
        MetaRow,
        "SELECT id, sort_timestamp, created_at FROM media_item"
    )
    .fetch_all(pool)
    .await
    .expect("media items must be readable");
    let mut key_by_id = HashMap::new();
    for row in rows {
        key_by_id.insert(row.id, (row.sort_timestamp, row.created_at));
    }

    for pair in ids.windows(2) {
        let (prev_ts, prev_created) = key_by_id[&pair[0]];
        let (next_ts, next_created) = key_by_id[&pair[1]];
        assert!(
            prev_ts < next_ts || (prev_ts == next_ts && prev_created <= next_created),
            "date-asc order violated between {} and {}",
            pair[0],
            pair[1]
        );
    }
}
