use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use chrono::{Duration, Utc};
use color_eyre::Result;
use color_eyre::eyre::{ContextCompat, bail};
use common_services::api::auth::interfaces::LoginUser;
use common_services::api::upload::interfaces::CheckHashesResponse;
use common_types::dev_constants::{EMAIL, PASSWORD};
use common_types::pb::api::TimelineSyncResponse;
use prost::Message;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_ENCODING};
use std::collections::HashSet;
use std::io::Cursor;

/// Calls `GET /api/timeline/sync` and decodes the compact protobuf response.
async fn sync(context: &TestContext, since: Option<i64>) -> Result<TimelineSyncResponse> {
    let mut request = context.http_client.get(format!(
        "{}/api/timeline/sync",
        context.settings.api.public_url
    ));
    if let Some(since) = since {
        request = request.query(&[("since", since)]);
    }
    let response = request.send().await?;
    if response.status() != StatusCode::OK {
        bail!("{} - {}", response.status(), response.text().await?);
    }
    Ok(TimelineSyncResponse::decode(Cursor::new(
        response.bytes().await?,
    ))?)
}

async fn live_ids(context: &TestContext) -> Result<HashSet<String>> {
    let ids: Vec<String> =
        sqlx::query_scalar!(r#"SELECT id FROM media_item WHERE deleted = false"#)
            .fetch_all(&context.pool)
            .await?;
    Ok(ids.into_iter().collect())
}

fn upserted_ids(response: &TimelineSyncResponse) -> HashSet<String> {
    response
        .upserted
        .iter()
        .map(|item| item.id.clone())
        .collect()
}

pub async fn test_mobile_sync_full_snapshot(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;

    // ACT
    let response = sync(context, None).await?;

    // ASSERT
    assert!(
        response.is_full_snapshot,
        "no `since` means a full snapshot"
    );
    assert!(response.sync_timestamp > 0);
    assert_eq!(response.deleted_ids, Vec::<String>::new());

    let db_ids = live_ids(context).await?;
    assert!(!db_ids.is_empty(), "test library must contain media");
    assert_eq!(db_ids, upserted_ids(&response));

    let item = response
        .upserted
        .first()
        .context("snapshot must contain media items")?;
    let expected = sqlx::query!(
        r#"
        SELECT
            (mi.width::REAL / mi.height::REAL) AS "ratio!",
            mi.is_video,
            mi.has_thumbnails AS "has_thumbnails!",
            mf.size_bytes,
            (EXTRACT(EPOCH FROM mi.sort_timestamp) * 1000)::BIGINT AS "sort_timestamp!",
            ROUND(
                EXTRACT(EPOCH FROM (mi.taken_at_local - (mi.sort_timestamp AT TIME ZONE 'UTC'))) / 60.0
            )::INT AS "timezone_offset_minutes!"
        FROM media_item mi
        JOIN media_features mf ON mf.media_item_id = mi.id
        WHERE mi.id = $1
        "#,
        item.id
    )
    .fetch_one(&context.pool)
    .await?;

    assert_eq!(item.sort_timestamp, expected.sort_timestamp);
    assert_eq!(item.size_bytes, expected.size_bytes);
    assert_eq!(item.is_video, expected.is_video);
    assert_eq!(item.has_thumbnails, expected.has_thumbnails);
    assert!((item.ratio - expected.ratio).abs() < 1e-6);
    assert_eq!(
        item.timezone_offset_minutes,
        expected.timezone_offset_minutes
    );

    // The protobuf response must be gzip-compressed when the client asks for it. A separate client
    // is used because reqwest's automatic gzip decoding (enabled by the unified feature set) strips
    // the `content-encoding` header from the response.
    let gzip_client = reqwest::Client::builder()
        .cookie_store(true)
        .no_gzip()
        .build()?;
    let response = gzip_client
        .post(format!(
            "{}/api/auth/login",
            context.settings.api.public_url
        ))
        .json(&LoginUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
        })
        .send()
        .await?;
    assert!(response.status().is_success());

    let compressed = gzip_client
        .get(format!(
            "{}/api/timeline/sync",
            context.settings.api.public_url
        ))
        .header(ACCEPT_ENCODING, "gzip")
        .send()
        .await?;
    assert_eq!(compressed.status(), StatusCode::OK);
    assert_eq!(
        compressed
            .headers()
            .get(CONTENT_ENCODING)
            .and_then(|value| value.to_str().ok()),
        Some("gzip"),
        "sync response must be gzip-compressed when requested"
    );

    Ok(())
}

pub async fn test_mobile_sync_delta(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let full = sync(context, None).await?;
    let mut ids = full.upserted.iter().map(|item| item.id.clone());
    let changed_id = ids.next().context("snapshot needs at least one item")?;
    let unchanged_id = ids.next().context("snapshot needs at least two items")?;
    let since = full.sync_timestamp - 60_000;

    sqlx::query!(
        "UPDATE media_item SET updated_at = now() WHERE id = $1",
        changed_id
    )
    .execute(&context.pool)
    .await?;
    sqlx::query!(
        "UPDATE media_item SET updated_at = to_timestamp(0) WHERE id = $1",
        unchanged_id
    )
    .execute(&context.pool)
    .await?;

    // ACT
    let delta = sync(context, Some(since)).await?;

    // ASSERT
    assert!(!delta.is_full_snapshot);
    let upserted = upserted_ids(&delta);
    assert!(
        upserted.contains(&changed_id),
        "items updated after `since` must be upserted"
    );
    assert!(
        !upserted.contains(&unchanged_id),
        "unmodified items must not be part of a delta"
    );
    assert!(!delta.deleted_ids.contains(&unchanged_id));

    Ok(())
}

pub async fn test_mobile_sync_soft_delete_and_restore(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let full = sync(context, None).await?;
    let id = full
        .upserted
        .first()
        .context("snapshot needs at least one item")?
        .id
        .clone();
    let since = full.sync_timestamp - 60_000;

    // ACT - move to bin
    let response = context
        .http_client
        .delete(format!(
            "{}/api/trash/soft-delete",
            context.settings.api.public_url
        ))
        .json(&serde_json::json!({ "ids": [id] }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let delta = sync(context, Some(since)).await?;

    // ASSERT
    assert!(
        delta.deleted_ids.contains(&id),
        "soft-deleted items must be reported as deleted"
    );
    assert!(!upserted_ids(&delta).contains(&id));

    // ACT - restore from bin
    let response = context
        .http_client
        .post(format!(
            "{}/api/trash/restore",
            context.settings.api.public_url
        ))
        .json(&serde_json::json!({ "ids": [id] }))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let restored = sync(context, Some(since)).await?;

    // ASSERT
    assert!(
        upserted_ids(&restored).contains(&id),
        "restored items must be upserted again"
    );
    assert!(!restored.deleted_ids.contains(&id));

    Ok(())
}

pub async fn test_mobile_sync_hard_delete_tombstone(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let full = sync(context, None).await?;
    let id = full
        .upserted
        .first()
        .context("snapshot needs at least one item")?
        .id
        .clone();
    let since = full.sync_timestamp - 60_000;

    // ACT - a direct delete fires the AFTER DELETE trigger that writes the tombstone
    sqlx::query!("DELETE FROM media_item WHERE id = $1", id)
        .execute(&context.pool)
        .await?;

    // ASSERT
    let tombstone: Option<String> = sqlx::query_scalar!(
        "SELECT media_item_id FROM deleted_media_item WHERE media_item_id = $1",
        id
    )
    .fetch_optional(&context.pool)
    .await?;
    assert_eq!(tombstone.as_deref(), Some(id.as_str()));

    let delta = sync(context, Some(since)).await?;
    assert!(
        delta.deleted_ids.contains(&id),
        "hard-deleted items must be reported through their tombstone"
    );
    assert!(!upserted_ids(&delta).contains(&id));

    Ok(())
}

pub async fn test_mobile_sync_retention_fallback(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;

    // ACT - epoch is far older than the tombstone retention window
    let response = sync(context, Some(0)).await?;

    // ASSERT
    assert!(
        response.is_full_snapshot,
        "a `since` older than tombstone retention must fall back to a full snapshot"
    );
    assert_eq!(live_ids(context).await?, upserted_ids(&response));

    Ok(())
}

pub async fn test_prune_deleted_media_items(context: &TestContext) -> Result<()> {
    // ARRANGE
    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user ORDER BY id LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let old_id = "oldtomb001";
    let recent_id = "newtomb001";
    for (id, days) in [(old_id, 120), (recent_id, 1)] {
        sqlx::query!(
            "INSERT INTO deleted_media_item (media_item_id, user_id, deleted_at) VALUES ($1, $2, $3)",
            id,
            user_id,
            Utc::now() - Duration::days(days)
        )
        .execute(&context.pool)
        .await?;
    }

    // ACT
    worker::handlers::clean_db::prune_deleted_media_items(&context.pool).await?;

    // ASSERT
    let remaining: HashSet<String> =
        sqlx::query_scalar!("SELECT media_item_id FROM deleted_media_item")
            .fetch_all(&context.pool)
            .await?
            .into_iter()
            .collect();
    assert!(
        !remaining.contains(old_id),
        "tombstones past retention must be pruned"
    );
    assert!(
        remaining.contains(recent_id),
        "recent tombstones must be kept"
    );

    sqlx::query!(
        "DELETE FROM deleted_media_item WHERE media_item_id = $1",
        recent_id
    )
    .execute(&context.pool)
    .await?;

    Ok(())
}

pub async fn test_check_hashes(context: &TestContext) -> Result<()> {
    // ARRANGE
    login(context).await?;
    let rows = sqlx::query!(
        "SELECT id, hash FROM media_item WHERE deleted = false ORDER BY created_at LIMIT 2"
    )
    .fetch_all(&context.pool)
    .await?;
    let first = rows.first().context("test library must contain media")?;
    let second = rows.get(1).context("test library must contain two items")?;

    // A soft-deleted item and a hash the server does not have.
    sqlx::query!(
        "UPDATE media_item SET deleted = true WHERE id = $1",
        second.id
    )
    .execute(&context.pool)
    .await?;
    let unknown_hash = "0".repeat(64);

    // ACT
    let response = context
        .http_client
        .post(format!(
            "{}/api/upload/check-hashes",
            context.settings.api.public_url
        ))
        .json(&serde_json::json!({
            "hashes": [first.hash.clone(), second.hash.clone(), unknown_hash.clone()],
        }))
        .send()
        .await?;

    // ASSERT
    assert_eq!(response.status(), StatusCode::OK);
    let body: CheckHashesResponse = response.json().await?;

    let first_ids = body
        .hashes
        .get(&first.hash)
        .context("known hash must be reported")?;
    assert!(first_ids.contains(&first.id));
    assert!(
        !first_ids.contains(&second.id),
        "soft-deleted items must not be reported as available"
    );
    assert!(
        !body.hashes.contains_key(&unknown_hash),
        "unknown hashes must not be reported"
    );

    // CLEANUP
    sqlx::query!(
        "UPDATE media_item SET deleted = false WHERE id = $1",
        second.id
    )
    .execute(&context.pool)
    .await?;

    Ok(())
}
