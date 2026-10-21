use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::{get_thumbnail_folders, require_media_mounted};
use app_state::IngestSettings;
use color_eyre::Result;
use common_services::database::jobs::Job;
use sqlx::PgPool;
use std::collections::HashSet;
use tokio::fs;
use tracing::warn;

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
        // Another cleanup path may have already removed the orphan.
        if let Err(e) = fs::remove_dir_all(&path_to_remove).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                "Couldn't remove orphan thumbnail dir {:?}: {:?}",
                &path_to_remove, e
            );
        }
    }

    Ok(())
}

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    if let Some(result) =
        require_media_mounted(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }
    clean_thumbnail_orphans(&context.pool, &context.settings.ingest).await?;
    Ok(JobResult::Done)
}
