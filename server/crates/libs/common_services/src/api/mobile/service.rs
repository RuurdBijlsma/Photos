use crate::api::app_error::AppError;
use crate::api::mobile::TOMBSTONE_RETENTION_DAYS;
use chrono::{DateTime, Duration, Utc};
use common_types::pb::api::{CompactTimelineItem, TimelineSyncResponse};
use sqlx::PgPool;

/// Compact timeline sync for mobile clients.
///
/// Without a `since` (or with one older than tombstone retention) this returns every live item as a
/// full snapshot. Otherwise it returns the items changed and the ids deleted after `since`.
pub async fn get_timeline_sync(
    user_id: i32,
    pool: &PgPool,
    since: Option<i64>,
) -> Result<TimelineSyncResponse, AppError> {
    let now = Utc::now();
    let since = since
        .map(|millis| {
            DateTime::from_timestamp_millis(millis).ok_or_else(|| {
                AppError::BadRequest("'since' is not a valid epoch millis timestamp.".to_string())
            })
        })
        .transpose()?;

    let is_full_snapshot =
        since.is_none_or(|since| since < now - Duration::days(TOMBSTONE_RETENTION_DAYS));

    let (upserted, deleted_ids) = match since {
        Some(since) if !is_full_snapshot => {
            let upserted = fetch_updated_items(user_id, pool, since).await?;
            let deleted_ids = fetch_deleted_ids(user_id, pool, since).await?;
            (upserted, deleted_ids)
        }
        _ => (fetch_all_items(user_id, pool).await?, Vec::new()),
    };

    Ok(TimelineSyncResponse {
        upserted,
        deleted_ids,
        sync_timestamp: now.timestamp_millis(),
        is_full_snapshot,
    })
}

async fn fetch_all_items(
    user_id: i32,
    pool: &PgPool,
) -> Result<Vec<CompactTimelineItem>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            mi.id,
            (EXTRACT(EPOCH FROM mi.sort_timestamp) * 1000)::BIGINT AS "sort_timestamp!",
            (mi.width::REAL / mi.height::REAL) AS "ratio!",
            mi.is_video,
            mi.has_thumbnails AS "has_thumbnails!",
            mf.size_bytes,
            mi.duration_ms::INT AS duration_ms,
            ROUND(
                EXTRACT(EPOCH FROM (mi.taken_at_local - (mi.sort_timestamp AT TIME ZONE 'UTC'))) / 60.0
            )::INT AS "timezone_offset_minutes!"
        FROM media_item mi
        JOIN media_features mf ON mf.media_item_id = mi.id
        WHERE mi.user_id = $1
          AND mi.deleted = false
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| CompactTimelineItem {
            id: r.id,
            sort_timestamp: r.sort_timestamp,
            ratio: r.ratio,
            is_video: r.is_video,
            has_thumbnails: r.has_thumbnails,
            size_bytes: r.size_bytes,
            duration_ms: r.duration_ms,
            timezone_offset_minutes: r.timezone_offset_minutes,
        })
        .collect())
}

async fn fetch_updated_items(
    user_id: i32,
    pool: &PgPool,
    since: DateTime<Utc>,
) -> Result<Vec<CompactTimelineItem>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            mi.id,
            (EXTRACT(EPOCH FROM mi.sort_timestamp) * 1000)::BIGINT AS "sort_timestamp!",
            (mi.width::REAL / mi.height::REAL) AS "ratio!",
            mi.is_video,
            mi.has_thumbnails AS "has_thumbnails!",
            mf.size_bytes,
            mi.duration_ms::INT AS duration_ms,
            ROUND(
                EXTRACT(EPOCH FROM (mi.taken_at_local - (mi.sort_timestamp AT TIME ZONE 'UTC'))) / 60.0
            )::INT AS "timezone_offset_minutes!"
        FROM media_item mi
        JOIN media_features mf ON mf.media_item_id = mi.id
        WHERE mi.user_id = $1
          AND mi.deleted = false
          AND mi.updated_at > $2
        "#,
        user_id,
        since
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| CompactTimelineItem {
            id: r.id,
            sort_timestamp: r.sort_timestamp,
            ratio: r.ratio,
            is_video: r.is_video,
            has_thumbnails: r.has_thumbnails,
            size_bytes: r.size_bytes,
            duration_ms: r.duration_ms,
            timezone_offset_minutes: r.timezone_offset_minutes,
        })
        .collect())
}

/// Soft-deletes (bin) and hard-deletes (tombstones) that happened after `since`.
async fn fetch_deleted_ids(
    user_id: i32,
    pool: &PgPool,
    since: DateTime<Utc>,
) -> Result<Vec<String>, AppError> {
    let ids = sqlx::query_scalar!(
        r#"
        SELECT id AS "id!"
        FROM media_item
        WHERE user_id = $1 AND deleted = true AND updated_at > $2
        UNION
        SELECT media_item_id AS "id!"
        FROM deleted_media_item
        WHERE user_id = $1 AND deleted_at > $2
        "#,
        user_id,
        since
    )
    .fetch_all(pool)
    .await?;

    Ok(ids)
}
