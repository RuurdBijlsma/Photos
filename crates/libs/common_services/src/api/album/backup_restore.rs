use crate::api::album::interfaces::{AlbumBackup, AlbumItemBackup, AlbumSort, BackupInfo};
use crate::api::album::service::get_representative_thumbnail;
use crate::api::app_error::AppError;
use crate::database::album::album::AlbumRole;
use crate::database::album_store::AlbumStore;
use crate::database::{CreateAlbumPayload, UpdateField};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::Path;

pub async fn list_backups(cache_root: &Path, user_id: i32) -> Result<Vec<BackupInfo>, AppError> {
    let backup_root = cache_root.join("albums").join(user_id.to_string());
    let mut backups = Vec::new();
    if !backup_root.exists() {
        return Ok(backups);
    }

    let mut entries = tokio::fs::read_dir(&backup_root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let metadata = entry.metadata().await?;
            let size_bytes = metadata.len();
            let modified = metadata.modified()?;
            let created_at = DateTime::<Utc>::from(modified);

            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                backups.push(BackupInfo {
                    filename: filename.to_string(),
                    size_bytes,
                    created_at,
                });
            }
        }
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    Ok(backups)
}

pub async fn backup_albums(pool: &PgPool, cache_root: &Path, user_id: i32) -> Result<(), AppError> {
    let albums = AlbumStore::list_by_user_id(pool, user_id).await?;
    let mut backups = Vec::new();

    for album in albums {
        let album_items = sqlx::query_as!(
            AlbumItemBackup,
            r#"
            SELECT media_item.hash, album_media_item.rank, album_media_item.added_at, album_media_item.added_by_user
            FROM album_media_item
            INNER JOIN media_item ON album_media_item.media_item_id = media_item.id
            WHERE album_media_item.album_id = $1
            "#,
            album.id
        )
            .fetch_all(pool)
            .await?;

        backups.push(AlbumBackup {
            id: album.id,
            name: album.name,
            description: album.description,
            items: album_items,
        });
    }

    let backup_root = cache_root.join("albums").join(user_id.to_string());
    tokio::fs::create_dir_all(&backup_root).await?;
    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let backup_path = backup_root.join(format!("{timestamp}.json"));
    let json = serde_json::to_string(&backups).map_err(std::io::Error::other)?;
    tokio::fs::write(backup_path, json).await?;

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn restore_albums(
    pool: &PgPool,
    user_id: i32,
    backup_path: &Path,
) -> Result<(), AppError> {
    if !backup_path.exists() {
        return Err(AppError::NotFound(
            backup_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default(),
        ));
    }
    // Load the backup contents
    let json_data = tokio::fs::read_to_string(&backup_path).await?;
    let backups: Vec<AlbumBackup> = serde_json::from_str(&json_data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Map hashes to active database IDs for the user
    let mut tx = pool.begin().await?;
    let media_items = sqlx::query!(
        "SELECT id, hash FROM media_item WHERE user_id = $1 AND deleted = false",
        user_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let hash_to_id: HashMap<String, String> =
        media_items.into_iter().map(|r| (r.hash, r.id)).collect();

    // Restore each album
    for backup in backups {
        let album_exists = AlbumStore::find_by_id(&mut *tx, &backup.id)
            .await?
            .is_some();
        if !album_exists {
            AlbumStore::create(
                &mut *tx,
                &backup.id,
                CreateAlbumPayload {
                    owner_id: user_id,
                    name: backup.name,
                    description: backup.description,
                    thumbnail_id: None,
                    sort_mode: AlbumSort::None,
                    is_public: false,
                },
            )
            .await?;
            AlbumStore::upsert_collaborator(&mut *tx, &backup.id, user_id, AlbumRole::Owner)
                .await?;
        }

        // Collect arrays to insert in a single query
        let mut batch_media_item_ids = Vec::new();
        let mut batch_ranks = Vec::new();
        let mut batch_added_ats = Vec::new();
        let mut batch_added_by_users = Vec::new();
        for item in backup.items {
            if let Some(media_item_id) = hash_to_id.get(&item.hash) {
                batch_media_item_ids.push(media_item_id.clone());
                batch_ranks.push(item.rank);
                batch_added_ats.push(item.added_at);
                batch_added_by_users.push(item.added_by_user);
            }
        }

        // Execute bulk insertion if there are items to restore
        if !batch_media_item_ids.is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO album_media_item (album_id, media_item_id, rank, added_at, added_by_user)
                SELECT
                    $1::TEXT,
                    vals.media_item_id,
                    vals.rank,
                    vals.added_at,
                    vals.added_by_user
                FROM UNNEST(
                    $2::TEXT[],
                    $3::DOUBLE PRECISION[],
                    $4::TIMESTAMPTZ[],
                    $5::INT[]
                ) AS vals(media_item_id, rank, added_at, added_by_user)
                ON CONFLICT (album_id, media_item_id)
                DO UPDATE SET
                    rank = EXCLUDED.rank,
                    added_at = EXCLUDED.added_at,
                    added_by_user = EXCLUDED.added_by_user
                "#,
                backup.id,
                &batch_media_item_ids,
                &batch_ranks,
                &batch_added_ats as &[DateTime<Utc>],
                &batch_added_by_users as &[Option<i32>]
            )
                .execute(&mut *tx)
                .await?;
        }

        // Apply a representative thumbnail if none is selected
        let album = AlbumStore::find_by_id(&mut *tx, &backup.id).await?;
        if let Some(album) = album
            && album.thumbnail_id.is_none()
            && !batch_media_item_ids.is_empty()
            && let Ok(Some(thumb_id)) =
                get_representative_thumbnail(&mut tx, &batch_media_item_ids).await
        {
            AlbumStore::update(
                &mut *tx,
                &backup.id,
                None,
                UpdateField::Ignore,
                UpdateField::Value(thumb_id),
                None,
            )
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}
