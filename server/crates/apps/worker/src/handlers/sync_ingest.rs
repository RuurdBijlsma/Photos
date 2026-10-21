use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::require_media_mounted;
use app_state::IngestSettings;
use color_eyre::Result;
use common_services::database::jobs::{Job, JobType};
use common_services::job_queue::enqueue_job;
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info, warn};

/// Reads the thumbnails directory and returns a set of all subdirectory names (media item IDs).
async fn get_thumbnail_folders(thumbnail_folder: &Path) -> Result<HashSet<String>> {
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

/// Synchronises thumbnail folders with the database, deleting orphans and re-ingesting incomplete items.
async fn sync_ingest(pool: &PgPool, settings: &IngestSettings) -> Result<()> {
    let thumbnails_root = &settings.thumbnails_root;

    let (thumb_ids, db_ids) = tokio::try_join!(get_thumbnail_folders(thumbnails_root), async {
        let rows: Vec<String> = sqlx::query_scalar!("SELECT id FROM media_item")
            .fetch_all(pool)
            .await?;
        Ok::<HashSet<String>, color_eyre::Report>(rows.into_iter().collect())
    })?;

    let to_delete: Vec<_> = thumb_ids.difference(&db_ids).cloned().collect();
    for id in to_delete {
        let path_to_remove = thumbnails_root.join(id);
        if let Err(e) = fs::remove_dir_all(&path_to_remove).await {
            warn!(
                "Couldn't remove dir {:?} in `sync_ingest` job: {:?}",
                &path_to_remove, e
            );
        }
    }

    repair_incomplete_ingest_with_thumb_ids(pool, settings, &thumb_ids).await?;

    Ok(())
}

async fn repair_incomplete_ingest_with_thumb_ids(
    pool: &PgPool,
    settings: &IngestSettings,
    thumb_ids: &HashSet<String>,
) -> Result<()> {
    let rows = sqlx::query!(
        r#"
        SELECT mi.id, mi.relative_path, mi.user_id, mi.has_thumbnails,
               NOT EXISTS (
                   SELECT 1 FROM visual_analysis va WHERE va.media_item_id = mi.id
               ) AS "needs_analysis!"
        FROM media_item mi
        WHERE mi.deleted = false AND mi.missing_since IS NULL
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut skipped = 0_usize;
    for row in rows {
        let needs_thumbnails = !row.has_thumbnails || !thumb_ids.contains(&row.id);
        if !needs_thumbnails && !row.needs_analysis {
            continue;
        }

        let file = settings.media_root.join(&row.relative_path);
        let file_exists = fs::metadata(&file)
            .await
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0);
        if !file_exists {
            debug!(
                "Skipping missing or empty file during ingest repair: {:?}",
                file
            );
            skipped += 1;
            continue;
        }

        if needs_thumbnails {
            enqueue_job::<()>(pool, settings, JobType::IngestThumbnails)
                .relative_path(row.relative_path.clone())
                .user_id(row.user_id)
                .call()
                .await?;
        }
        if row.needs_analysis {
            enqueue_job::<()>(pool, settings, JobType::IngestAnalysis)
                .relative_path(row.relative_path.clone())
                .user_id(row.user_id)
                .call()
                .await?;
        }
    }

    if skipped > 0 {
        info!("Skipped {skipped} missing or empty files during ingest repair.");
    }

    Ok(())
}

pub async fn repair_incomplete_ingest(pool: &PgPool, settings: &IngestSettings) -> Result<()> {
    let thumb_ids = get_thumbnail_folders(&settings.thumbnails_root).await?;
    repair_incomplete_ingest_with_thumb_ids(pool, settings, &thumb_ids).await
}

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    if let Some(result) =
        require_media_mounted(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }
    sync_ingest(&context.pool, &context.settings.ingest).await?;
    Ok(JobResult::Done)
}
