use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::require_media_mounted_fresh;
use app_state::IngestSettings;
use color_eyre::Result;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::media_reconcile;
use sqlx::PgPool;
use std::collections::HashSet;
use std::io::ErrorKind;
use tracing::info;

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    prune_missing_ingest_jobs(&context.pool, &context.settings.ingest).await
}

pub async fn prune_missing_ingest_jobs(
    pool: &PgPool,
    settings: &IngestSettings,
) -> Result<JobResult> {
    // Soft-deleted rows must be cancelled even if the media drive is unavailable.
    let cancelled_soft_deleted = sqlx::query!(
        r#"
        UPDATE jobs j
        SET status = 'cancelled'
        FROM media_item mi
        WHERE mi.deleted = true
          AND j.relative_path = mi.relative_path
          AND j.status IN ('queued', 'running')
          AND j.job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')
        "#
    )
    .execute(pool)
    .await?
    .rows_affected();

    if let Some(result) = require_media_mounted_fresh(pool, &settings.media_root).await? {
        return Ok(result);
    }

    let rows = sqlx::query!(
        r#"
        SELECT j.relative_path, j.user_id, u.media_folder
        FROM jobs j
        LEFT JOIN app_user u ON u.id = j.user_id
        WHERE j.status IN ('queued', 'running')
          AND j.job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')
          AND j.relative_path IS NOT NULL
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut seen = HashSet::new();
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for row in rows {
        let (Some(relative_path), Some(user_id), Some(media_folder)) =
            (row.relative_path, row.user_id, row.media_folder)
        else {
            continue;
        };
        if !seen.insert((relative_path.clone(), user_id)) {
            continue;
        }
        if !settings.media_root.join(&media_folder).is_dir() {
            continue;
        }
        match tokio::fs::metadata(settings.media_root.join(&relative_path)).await {
            Ok(_) => present.push(relative_path),
            Err(e) if e.kind() == ErrorKind::NotFound => missing.push(relative_path),
            Err(e) => info!("Skipping {relative_path} during ingest sweep: {e}"),
        }
    }

    if !present.is_empty() {
        MediaItemStore::unmark_relative_paths_as_missing(pool, &present).await?;
    }
    let removed = if missing.is_empty() {
        media_reconcile::ReconcileRemovalResult::default()
    } else {
        media_reconcile::reconcile_removed_paths(pool, settings, &missing).await?
    };

    info!(
        "Ingest sweep: cancelled {cancelled_soft_deleted} soft-deleted job(s), restored {} path(s), marked {} path(s) missing",
        present.len(),
        removed.marked_missing
    );
    Ok(JobResult::Done)
}
