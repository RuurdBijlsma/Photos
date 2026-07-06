use crate::api::app_error::AppError;
use crate::api::photos::removal::delete_item_and_thumbnails;
use app_state::constants;
use common_types::pb::api::{OrderedMediaResponse, SimpleTimelineItem};
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::info;

/// Fetches all soft-deleted media items for the user, sorted by `sort_timestamp` descending.
pub async fn get_trash_items(
    pool: &PgPool,
    user_id: i32,
) -> Result<OrderedMediaResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            is_video,
            has_thumbnails,
            duration_ms::INT AS duration_ms,
            (width::real / height::real) AS "ratio!"
        FROM media_item
        WHERE user_id = $1 AND deleted = true
        ORDER BY sort_timestamp DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|r| SimpleTimelineItem {
            id: r.id,
            is_video: r.is_video,
            has_thumbnails: r.has_thumbnails,
            duration_ms: r.duration_ms,
            ratio: r.ratio,
        })
        .collect();

    Ok(OrderedMediaResponse { items })
}

/// Soft-deletes a list of media items.
pub async fn soft_delete_items(
    pool: &PgPool,
    user_id: i32,
    ids: &[String],
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
            UPDATE media_item
            SET deleted = true
            WHERE id = ANY($1) AND user_id = $2
        "#,
        ids,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Restores a list of soft-deleted media items.
pub async fn restore_items(pool: &PgPool, user_id: i32, ids: &[String]) -> Result<(), AppError> {
    sqlx::query!(
        r#"
            UPDATE media_item
            SET deleted = false
            WHERE id = ANY($1) AND user_id = $2
        "#,
        ids,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Permanently deletes media items from the database and filesystem.
///
/// Item must be soft-deleted before hard-deleting, otherwise it will error.
pub async fn perma_delete_items(
    pool: &PgPool,
    user_id: i32,
    ids: &[String],
    media_root: &Path,
    thumbnail_root: &Path,
) -> Result<(), AppError> {
    if !constants().allow_file_deletion {
        return Err(AppError::Forbidden(
            "File deletion not permitted.".to_owned(),
        ));
    }
    let items = sqlx::query!(
        r#"
        SELECT id, relative_path
        FROM media_item
        WHERE id = ANY($1) AND user_id = $2 AND deleted = true
        "#,
        ids,
        user_id
    )
    .fetch_all(pool)
    .await?;

    // Check if all requested ids are soft-deleted already
    let unique_requested_count = ids.iter().collect::<HashSet<_>>().len();
    if items.len() < unique_requested_count {
        return Err(AppError::BadRequest(
            "One or more requested media items are not soft-deleted, do not exist, or do not belong to you."
                .to_string(),
        ));
    }

    // Delete original files from disk and then clean up DB and thumbnails
    for item in items {
        let file_path = media_root.join(&item.relative_path);
        if file_path.exists() {
            info!("Removing original media file: {}", file_path.display());
            if let Err(e) = fs::remove_file(&file_path).await {
                // Return an error if we fail to remove an existing file.
                // If the file is already missing, delete_item_and_thumbnails will still clean up the DB.
                return Err(AppError::Internal(color_eyre::eyre::eyre!(
                    "Failed to delete original file {}: {}",
                    item.relative_path,
                    e
                )));
            }
        }
        // Clean up database records and thumbnails folder
        delete_item_and_thumbnails(pool, thumbnail_root, &item.relative_path).await?;
    }

    Ok(())
}
