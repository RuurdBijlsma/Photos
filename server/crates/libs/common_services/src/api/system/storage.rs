use crate::api::app_error::AppError;
use crate::api::system::storage_helpers::get_folder_size;
use app_state::IngestSettings;
use common_types::pb::api::{StorageReviewItem, StorageReviewResponse, StorageSummaryResponse};
use moka::future::Cache;
use sqlx::PgPool;
use std::sync::OnceLock;
use std::time::Duration;

const REVIEW_LIMIT: i64 = 250;
const REVIEW_MIN_SIZE_BYTES: i64 = 10 * 1024 * 1024;
const BLURRY_QUALITY_THRESHOLD: f64 = 50.0;

static SIZE_CACHE: OnceLock<Cache<String, (u64, u64)>> = OnceLock::new();

fn get_size_cache() -> &'static Cache<String, (u64, u64)> {
    SIZE_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(10)
            .time_to_live(Duration::from_mins(2))
            .build()
    })
}

pub async fn get_storage_summary(
    pool: &PgPool,
    settings: &IngestSettings,
    user_id: i32,
) -> Result<StorageSummaryResponse, AppError> {
    let large_task = sqlx::query!(
        r#"
        SELECT
            COALESCE(SUM(size_bytes), 0)::BIGINT AS "large_potential_savings!",
            COUNT(*)::INT AS "large_item_count!"
        FROM (
            SELECT mf.size_bytes
            FROM media_item mi
            JOIN media_features mf ON mf.media_item_id = mi.id
            WHERE mi.user_id = $1
              AND mi.deleted = false
              AND mf.size_bytes > $2
            ORDER BY mf.size_bytes DESC
            LIMIT $3
        ) large_items
        "#,
        user_id,
        REVIEW_MIN_SIZE_BYTES,
        REVIEW_LIMIT,
    )
    .fetch_one(pool);

    let blurry_task = sqlx::query!(
        r#"
        SELECT
            COALESCE(SUM(size_bytes), 0)::BIGINT AS "blurry_potential_savings!",
            COUNT(*)::INT AS "blurry_item_count!"
        FROM (
            SELECT DISTINCT mi.id, mf.size_bytes
            FROM media_item mi
            JOIN media_features mf ON mf.media_item_id = mi.id
            JOIN visual_analysis va ON va.media_item_id = mi.id
            JOIN measured_quality mq ON mq.visual_analysis_id = va.id
            WHERE mi.user_id = $1
              AND mi.deleted = false
              AND mi.is_video = false
              AND mq.weighted_score < $2
        ) blurry_items
        "#,
        user_id,
        BLURRY_QUALITY_THRESHOLD,
    )
    .fetch_one(pool);

    let media_folder = settings.media_root.clone();
    let app_data_folder = settings.app_data_root.clone();

    // Use moka cache with TTL
    let cache = get_size_cache();
    let (media_folder_size, app_data_folder_size) = cache
        .get_with("sizes_key".to_string(), async move {
            let res = tokio::task::spawn_blocking(move || {
                let media = get_folder_size(&media_folder);
                let app_data = get_folder_size(&app_data_folder);
                (media, app_data)
            })
            .await;
            res.unwrap_or((0, 0))
        })
        .await;

    let (large_row, blurry_row) =
        tokio::try_join!(async { large_task.await.map_err(AppError::from) }, async {
            blurry_task.await.map_err(AppError::from)
        })?;

    Ok(StorageSummaryResponse {
        large_potential_savings: large_row.large_potential_savings,
        large_item_count: large_row.large_item_count,
        blurry_potential_savings: blurry_row.blurry_potential_savings,
        blurry_item_count: blurry_row.blurry_item_count,
        media_folder_size_bytes: media_folder_size as i64,
        app_data_folder_size_bytes: app_data_folder_size as i64,
    })
}

pub async fn get_large_storage_items(
    pool: &PgPool,
    user_id: i32,
) -> Result<StorageReviewResponse, AppError> {
    let items = sqlx::query_as!(
        StorageReviewItem,
        r#"
        SELECT
            mi.id AS "id!",
            mi.is_video AS "is_video!",
            mi.filename AS "filename!",
            mi.has_thumbnails AS "has_thumbnails!",
            mi.duration_ms::INT AS "duration_ms",
            (mi.width::REAL / mi.height::REAL) AS "ratio!",
            mf.size_bytes AS "size_bytes!",
            mi.taken_at_local::TEXT AS "taken_at_local!",
            NULL::REAL AS "weighted_score"
        FROM media_item mi
        JOIN media_features mf ON mf.media_item_id = mi.id
        WHERE mi.user_id = $1
          AND mi.deleted = false
          AND mf.size_bytes > $2
        ORDER BY mf.size_bytes DESC
        LIMIT $3
        "#,
        user_id,
        REVIEW_MIN_SIZE_BYTES,
        REVIEW_LIMIT,
    )
    .fetch_all(pool)
    .await?;

    let total_size = items.iter().map(|r| r.size_bytes).sum::<i64>();
    Ok(StorageReviewResponse { items, total_size })
}

pub async fn get_blurry_storage_items(
    pool: &PgPool,
    user_id: i32,
) -> Result<StorageReviewResponse, AppError> {
    let items = sqlx::query_as!(
        StorageReviewItem,
        r#"
        SELECT
            id AS "id!",
            is_video AS "is_video!",
            filename AS "filename!",
            has_thumbnails AS "has_thumbnails!",
            duration_ms AS "duration_ms",
            ratio AS "ratio!",
            size_bytes AS "size_bytes!",
            taken_at_local::TEXT AS "taken_at_local!",
            weighted_score AS "weighted_score"
        FROM (
            SELECT DISTINCT ON (mi.id)
                mi.id,
                mi.is_video,
                mi.filename,
                mi.has_thumbnails,
                mi.duration_ms::INT AS duration_ms,
                (mi.width::REAL / mi.height::REAL) AS ratio,
                mf.size_bytes,
                mi.taken_at_local,
                mq.weighted_score::REAL AS weighted_score
            FROM media_item mi
            JOIN media_features mf ON mf.media_item_id = mi.id
            JOIN visual_analysis va ON va.media_item_id = mi.id
            JOIN measured_quality mq ON mq.visual_analysis_id = va.id
            WHERE mi.user_id = $1
              AND mi.deleted = false
              AND mi.is_video = false
              AND mq.weighted_score < $2
            ORDER BY mi.id, mq.weighted_score ASC
        ) items
        ORDER BY size_bytes DESC
        "#,
        user_id,
        BLURRY_QUALITY_THRESHOLD,
    )
    .fetch_all(pool)
    .await?;

    let total_size = items.iter().map(|r| r.size_bytes).sum::<i64>();
    Ok(StorageReviewResponse { items, total_size })
}
