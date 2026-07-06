use crate::database::media_item_store::MediaItemStore;
use color_eyre::Result;
use sqlx::PgPool;
use std::path::Path;
use tokio::fs;
use tracing::info;

/// Fully removes a media item from the database and its associated thumbnails from disk.
///
/// This does NOT touch the original media file on the filesystem.
pub async fn delete_item_and_thumbnails(
    pool: &PgPool,
    thumbnail_root: &Path,
    relative_path: &str,
) -> Result<()> {
    // 1. Delete from database and get the ID for thumbnail cleanup.
    let deleted_id = MediaItemStore::delete_by_relative_path(pool, relative_path).await?;

    // 2. Perform thumbnail cleanup if the item was found.
    if let Some(id) = deleted_id {
        let thumb_dir = thumbnail_root.join(id);
        if thumb_dir.exists() {
            info!("Removing thumbnails for media item {}", relative_path);
            fs::remove_dir_all(thumb_dir).await?;
        }
    }

    Ok(())
}
