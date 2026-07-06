use crate::context::WorkerContext;
use crate::handlers::JobResult;
use app_state::IngestSettings;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::alert;
use common_services::database::jobs::Job;
use common_services::database::user_store::UserStore;
use common_services::job_queue::enqueue_full_ingest;
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::info;
use tracing::warn;

/// Reads the thumbnails directory and returns a set of all subdirectory names (media item IDs).
async fn get_thumbnail_folders(thumbnail_folder: &Path) -> Result<HashSet<String>> {
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

/// Synchronises thumbnail folders with the database, deleting orphans and re-ingesting items with missing thumbnails.
async fn sync_thumbnails(pool: &PgPool, settings: &IngestSettings) -> Result<()> {
    let Some(job_count) = sqlx::query_scalar!(
        "SELECT count(id) FROM jobs WHERE status IN ('running', 'queued') AND job_type in ('ingest_thumbnails', 'remove')"
    )
        .fetch_one(pool)
        .await?
    else {
        return Err(eyre!("Can't get job count"));
    };
    if job_count > 0 {
        return Ok(()); // skip if ingest jobs are pending
    }

    let thumbnails_root = &settings.thumbnails_root;
    let media_root = &settings.media_root;

    let (thumb_ids, db_ids) = tokio::try_join!(get_thumbnail_folders(thumbnails_root), async {
        let rows: Vec<String> = sqlx::query_scalar!("SELECT id FROM media_item")
            .fetch_all(pool)
            .await?;
        Ok::<HashSet<String>, color_eyre::Report>(rows.into_iter().collect())
    })?;

    let to_delete: Vec<_> = thumb_ids.difference(&db_ids).cloned().collect();
    for id in to_delete {
        fs::remove_dir_all(thumbnails_root.join(id)).await?;
    }

    let db_items_missing_thumbnails = db_ids.difference(&thumb_ids).cloned().collect::<Vec<_>>();
    for id in db_items_missing_thumbnails {
        let relative_path: String =
            sqlx::query_scalar!("SELECT relative_path FROM media_item WHERE id = $1", id)
                .fetch_one(pool)
                .await?;
        let file = media_root.join(&relative_path);
        if file.exists() {
            info!("Media item has no thumbnail, re-ingesting now. {:?}", file);
            // Re-ingest files with missing thumbnails, as long as the fs file exists.

            if let Some(user) = UserStore::find_user_by_relative_path(pool, &relative_path).await? {
                enqueue_full_ingest(pool, settings, &relative_path, user.id, None).await?;
            } else {
                alert!("[Sync - Thumbnail scan] Cannot find user from relative path.");
            }
        }
    }

    Ok(())
}

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    sync_thumbnails(&context.pool, &context.settings.ingest).await?;
    Ok(JobResult::Done)
}
