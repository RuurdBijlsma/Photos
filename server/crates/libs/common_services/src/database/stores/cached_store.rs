use crate::database::DbError;
use crate::database::user_store::UserStore;
use moka::future::Cache;
use sqlx::{Executor, Postgres};
use std::sync::OnceLock;
use std::time::Duration;

/// Helper macro to handle the "Check Cache -> Fetch DB on Miss -> Write Cache" boilerplate
/// safely without requiring database errors to implement `Clone`.
macro_rules! cache_lookup {
    ($cache:expr, $key:expr, $db_future:expr) => {{
        if let Some(cached) = $cache.get(&$key).await {
            Ok(cached)
        } else {
            let fresh = $db_future.await?;
            $cache.insert($key, fresh.clone()).await;
            Ok(fresh)
        }
    }};
}

pub static CACHED_STORE: OnceLock<CachedStore> = OnceLock::new();
#[must_use]
pub fn cached_store() -> &'static CachedStore {
    CACHED_STORE.get_or_init(CachedStore::new)
}

#[derive(Clone, Debug)]
pub struct CachedMediaMetadata {
    pub relative_path: String,
    pub user_id: i32,
}

#[derive(Clone)]
pub struct CachedStore {
    media_items: Cache<String, CachedMediaMetadata>,
    user_folders: Cache<i32, Option<String>>,
}

impl Default for CachedStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            media_items: Cache::builder()
                .time_to_live(Duration::from_mins(10))
                .max_capacity(10_000)
                .build(),
            user_folders: Cache::builder()
                .time_to_live(Duration::from_mins(10))
                .max_capacity(1_000)
                .build(),
        }
    }

    /// Retrieves a media item's metadata, falling back to database and writing to cache on miss.
    pub async fn get_rel_path_and_user_id_for_media_item(
        &self,
        executor: impl Executor<'_, Database = Postgres>,
        media_item_id: &str,
    ) -> Result<CachedMediaMetadata, DbError> {
        let key = media_item_id.to_string();

        cache_lookup!(self.media_items, key, async {
            let item = sqlx::query!(
                r#"
                SELECT relative_path, user_id
                FROM media_item
                WHERE id = $1
                "#,
                media_item_id
            )
            .fetch_one(executor)
            .await?;
            Ok::<CachedMediaMetadata, DbError>(CachedMediaMetadata {
                relative_path: item.relative_path,
                user_id: item.user_id,
            })
        })
    }

    /// Retrieves the assigned media folder for a user ID.
    pub async fn get_user_media_folder(
        &self,
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
    ) -> Result<Option<String>, DbError> {
        let key = user_id;

        cache_lookup!(self.user_folders, key, async {
            let user = UserStore::get_user_media_folder(executor, user_id).await?;
            Ok::<Option<String>, DbError>(user)
        })
    }

    pub async fn invalidate_media_item(&self, media_item_id: &str) {
        self.media_items.invalidate(media_item_id).await;
    }

    pub async fn invalidate_user_folder(&self, user_id: i32) {
        self.user_folders.invalidate(&user_id).await;
    }
}
