use crate::database::stores::album_store::AlbumStore;
use crate::database::{DbError, UpdateField};
use sqlx::{Executor, PgTransaction, Postgres};

pub struct TakeoutLinkStore;

/// Filenames with any takeout link row for the given user.
pub struct TakeoutLinkFilenameRow {
    pub filename: String,
    pub album_id: String,
    pub media_item_id: Option<String>,
}

impl TakeoutLinkStore {
    /// Bulk-inserts pending link rows (one album-folder occurrence per filename).
    pub async fn insert_pending(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
        batch_id: &str,
        album_id: &str,
        filenames: &[String],
    ) -> Result<(), DbError> {
        if filenames.is_empty() {
            return Ok(());
        }
        sqlx::query!(
            r#"
            INSERT INTO takeout_album_link (user_id, batch_id, album_id, filename)
            SELECT $1, $2, $3, u.filename
            FROM UNNEST($4::TEXT[]) AS u(filename)
            ON CONFLICT (user_id, album_id, filename) DO NOTHING
            "#,
            user_id,
            batch_id,
            album_id,
            filenames
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Inserts (or completes) a link row that is already attached to an existing media item.
    pub async fn insert_consumed(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
        batch_id: &str,
        album_id: &str,
        filename: &str,
        media_item_id: &str,
    ) -> Result<(), DbError> {
        sqlx::query!(
            r#"
            INSERT INTO takeout_album_link (user_id, batch_id, album_id, filename, media_item_id)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, album_id, filename)
            DO UPDATE SET media_item_id = EXCLUDED.media_item_id
            "#,
            user_id,
            batch_id,
            album_id,
            filename,
            media_item_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Consumes all pending links for `(user_id, filename)` by attaching them to the
    /// given media item, returning the album ids that were consumed.
    ///
    /// Called from `IngestMetadata` inside the same transaction as the media item
    /// upsert, so membership is attached atomically with the item itself, no matter
    /// in which order takeout zips were uploaded or processed.
    pub async fn consume_pending(
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        filename: &str,
        media_item_id: &str,
    ) -> Result<Vec<String>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            UPDATE takeout_album_link
            SET media_item_id = $3
            WHERE user_id = $1 AND filename = $2 AND media_item_id IS NULL
            RETURNING album_id
            "#,
            user_id,
            filename,
            media_item_id
        )
        .fetch_all(&mut **tx)
        .await?)
    }

    /// All link rows for the given filenames (needed to know which filenames a
    /// previous takeout pass already claimed, and for which item).
    pub async fn find_by_filenames(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
        filenames: &[String],
    ) -> Result<Vec<TakeoutLinkFilenameRow>, DbError> {
        if filenames.is_empty() {
            return Ok(Vec::new());
        }
        Ok(sqlx::query_as!(
            TakeoutLinkFilenameRow,
            r#"
            SELECT filename, album_id, media_item_id
            FROM takeout_album_link
            WHERE user_id = $1 AND filename = ANY($2)
            "#,
            user_id,
            filenames
        )
        .fetch_all(executor)
        .await?)
    }

    /// Deletes pending links that were never consumed (their ingest failed
    /// permanently), so a later re-import can pick those files up again.
    pub async fn delete_stale_pending(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
    ) -> Result<(), DbError> {
        sqlx::query!(
            r#"
            DELETE FROM takeout_album_link
            WHERE user_id = $1 AND media_item_id IS NULL
              AND created_at < now() - INTERVAL '7 days'
            "#,
            user_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

/// Attaches an ingested media item to the given albums, giving each album a
/// thumbnail if it does not have one yet (first item wins).
pub async fn attach_media_item_to_albums(
    tx: &mut PgTransaction<'_>,
    user_id: i32,
    item_id: &str,
    album_ids: &[String],
) -> Result<(), DbError> {
    let single_item = [item_id.to_owned()];
    for album_id in album_ids {
        AlbumStore::add_media_items(&mut **tx, album_id, &single_item, user_id).await?;
        if let Some(album) = AlbumStore::find_by_id(&mut **tx, album_id).await?
            && album.thumbnail_id.is_none()
        {
            AlbumStore::update(
                &mut **tx,
                album_id,
                None,
                UpdateField::Ignore,
                UpdateField::Value(item_id.to_owned()),
                None,
            )
            .await?;
        }
    }
    Ok(())
}
