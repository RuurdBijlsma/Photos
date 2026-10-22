use crate::context::WorkerContext;
use crate::handlers::JobResult;
use app_state::IngestSettings;
use color_eyre::Result;
use common_services::database::jobs::Job;
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::warn;

/// Reads the thumbnails directory and returns a set of all subdirectory names (media item IDs).
pub(crate) async fn get_thumbnail_folders(thumbnail_folder: &Path) -> Result<HashSet<String>> {
    if !thumbnail_folder.exists() {
        fs::create_dir_all(thumbnail_folder).await?;
    }
    let mut set = HashSet::new();
    let mut entries = fs::read_dir(thumbnail_folder).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            set.insert(name.to_owned());
        }
    }
    Ok(set)
}

/// Deletes thumbnail folders that do not belong to any media item.
pub async fn clean_thumbnail_orphans(pool: &PgPool, settings: &IngestSettings) -> Result<()> {
    let thumbnails_root = &settings.thumbnails_root;
    let (thumb_ids, db_ids) = tokio::try_join!(get_thumbnail_folders(thumbnails_root), async {
        let rows: Vec<String> = sqlx::query_scalar!("SELECT id FROM media_item")
            .fetch_all(pool)
            .await?;
        Ok::<HashSet<String>, color_eyre::Report>(rows.into_iter().collect())
    })?;

    for id in thumb_ids.difference(&db_ids) {
        let path_to_remove = thumbnails_root.join(id);
        if let Err(e) = fs::remove_dir_all(&path_to_remove).await {
            warn!(
                "Couldn't remove orphan thumbnail dir {:?}: {:?}",
                &path_to_remove, e
            );
        }
    }
    Ok(())
}

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    clean_thumbnail_orphans(&context.pool, &context.settings.ingest).await?;
    Ok(JobResult::Done)
}
