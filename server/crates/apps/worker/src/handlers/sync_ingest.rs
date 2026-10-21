use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::{get_thumbnail_folders, require_media_mounted};
use app_state::IngestSettings;
use color_eyre::Result;
use common_services::database::jobs::{Job, JobType};
use common_services::job_queue::enqueue_job;
use sqlx::PgPool;
use std::collections::HashSet;
use tokio::fs;
use tracing::{debug, info};

async fn repair_incomplete_ingest_with_thumb_ids(
    pool: &PgPool,
    settings: &IngestSettings,
    thumb_ids: &HashSet<String>,
    user_id: Option<i32>,
) -> Result<()> {
    let rows = sqlx::query!(
        r#"
        SELECT mi.id, mi.relative_path, mi.user_id, mi.has_thumbnails,
               mi.thumbnail_repair_blocked, mi.analysis_repair_blocked,
               NOT EXISTS (
                   SELECT 1 FROM visual_analysis va WHERE va.media_item_id = mi.id
               ) AS "needs_analysis!"
        FROM media_item mi
        WHERE mi.deleted = false AND mi.missing_since IS NULL
          AND ($1::int IS NULL OR mi.user_id = $1)
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut skipped = 0_usize;
    for row in rows {
        let needs_thumbnails = !row.has_thumbnails || !thumb_ids.contains(&row.id);
        let repair_thumbnails = needs_thumbnails && !row.thumbnail_repair_blocked;
        // Analysis reads generated thumbnails, so a blocked thumbnail stage blocks it too.
        let repair_analysis = row.needs_analysis
            && !row.analysis_repair_blocked
            && (!needs_thumbnails || !row.thumbnail_repair_blocked);
        if !repair_thumbnails && !repair_analysis {
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

        if repair_thumbnails {
            enqueue_job::<()>(pool, settings, JobType::IngestThumbnails)
                .relative_path(row.relative_path.clone())
                .user_id(row.user_id)
                .call()
                .await?;
        }
        if repair_analysis {
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

pub async fn repair_incomplete_ingest(
    pool: &PgPool,
    settings: &IngestSettings,
    user_id: Option<i32>,
) -> Result<()> {
    let thumb_ids = get_thumbnail_folders(&settings.thumbnails_root).await?;
    repair_incomplete_ingest_with_thumb_ids(pool, settings, &thumb_ids, user_id).await
}

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    if let Some(result) =
        require_media_mounted(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }
    repair_incomplete_ingest(&context.pool, &context.settings.ingest, job.user_id).await?;
    Ok(JobResult::Done)
}
