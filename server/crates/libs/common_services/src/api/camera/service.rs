use crate::api::app_error::AppError;
use crate::api::camera::interfaces::CameraSummary;
use common_types::pb::api::{
    CameraInfo, FullCameraPhotosResponse, ListCameraResponse, SimpleTimelineItem,
};
use sqlx::PgPool;
use tracing::instrument;

#[instrument(skip(pool))]
pub async fn get_all_cameras(pool: &PgPool, user_id: i32) -> Result<ListCameraResponse, AppError> {
    let cameras = sqlx::query_as!(
        CameraSummary,
        r#"
            WITH camera_groups AS (
                SELECT
                    c.media_item_id,
                    TRIM(c.camera_make) AS raw_make,
                    TRIM(c.camera_model) AS raw_model,
                    LOWER(TRIM(c.camera_make)) AS norm_make,
                    LOWER(REGEXP_REPLACE(TRIM(c.camera_model), '\s*\(.*\)$', '', 'i')) AS norm_model,
                    m.sort_timestamp,
                    m.has_thumbnails
                FROM camera_settings c
                INNER JOIN media_item m ON c.media_item_id = m.id
                WHERE m.user_id = $1
                  AND m.deleted = false
                  AND c.camera_make IS NOT NULL
                  AND c.camera_model IS NOT NULL
                  AND c.camera_make != '--'
                  AND c.camera_model != '--'
                  AND LOWER(TRIM(c.camera_make)) != 'unknown'
            ),
            camera_stats AS (
                SELECT
                    norm_make,
                    norm_model,
                    MIN(raw_make) AS camera_make,
                    REGEXP_REPLACE(MIN(raw_model), '\s*\(.*\)$', '', 'i') AS camera_model,
                    COUNT(*)::INT AS count
                FROM camera_groups
                GROUP BY norm_make, norm_model
            ),
            ranked_photos AS (
                SELECT
                    norm_make,
                    norm_model,
                    media_item_id,
                    ROW_NUMBER() OVER (
                        PARTITION BY norm_make, norm_model
                        ORDER BY has_thumbnails DESC, sort_timestamp DESC
                    ) as rn
                FROM camera_groups
            )
            SELECT
                s.camera_make AS "camera_make!",
                s.camera_model AS "camera_model!",
                s.count AS "count!",
                r.media_item_id AS "thumbnail_id!"
            FROM camera_stats s
            INNER JOIN ranked_photos r
                ON s.norm_make = r.norm_make AND s.norm_model = r.norm_model
            WHERE r.rn = 1
            ORDER BY s.count DESC;
            "#,
        user_id
    )
        .fetch_all(pool)
        .await?;

    let camera_pb = cameras
        .into_iter()
        .map(|p| CameraInfo {
            make: p.camera_make,
            model: p.camera_model,
            photo_count: p.count,
            thumbnail_id: p.thumbnail_id,
        })
        .collect();

    Ok(ListCameraResponse { cameras: camera_pb })
}

#[instrument(skip(pool))]
pub async fn get_camera_photos(
    pool: &PgPool,
    camera_make: &str,
    camera_model: &str,
    user_id: i32,
) -> Result<FullCameraPhotosResponse, AppError> {
    let items = sqlx::query_as!(
        SimpleTimelineItem,
        r#"
            SELECT
                mi.id,
                mi.is_video,
                mi.has_thumbnails,
                mi.duration_ms::INT AS "duration_ms",
                (mi.width::real / mi.height::real) AS "ratio!"
            FROM camera_settings c
            INNER JOIN media_item mi ON c.media_item_id = mi.id
            WHERE mi.user_id = $1
              AND mi.deleted = false
              AND LOWER(TRIM(c.camera_make)) = LOWER(TRIM($2))
              AND LOWER(REGEXP_REPLACE(TRIM(c.camera_model), '\s*\(.*\)$', '', 'i')) = LOWER(TRIM($3))
            ORDER BY mi.sort_timestamp DESC;
        "#,
        user_id,
        camera_make,
        camera_model
    )
        .fetch_all(pool)
        .await?;

    let photo_count = items.len() as i32;

    let Some(first_item) = items.first().map(|i| i.id.clone()) else {
        return Err(AppError::NotFound(
            "No photos found for this camera".to_owned(),
        ));
    };

    Ok(FullCameraPhotosResponse {
        camera: Some(CameraInfo {
            photo_count,
            make: camera_make.to_string(),
            model: camera_model.to_string(),
            thumbnail_id: first_item,
        }),
        items,
    })
}
