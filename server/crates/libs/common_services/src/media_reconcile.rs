use crate::database::DbError;
use app_state::IngestSettings;
use sqlx::PgPool;
use std::path::Path;

#[derive(Debug, Default)]
pub struct ReconcileRemovalResult {
    pub marked_missing: usize,
    pub cancelled_jobs: u64,
}

/// Marks removed files and directory descendants missing and cancels their active ingest jobs.
///
/// # Errors
///
/// Returns an error if the database transaction fails.
pub async fn reconcile_removed_paths(
    pool: &PgPool,
    settings: &IngestSettings,
    relative_paths: &[String],
) -> Result<ReconcileRemovalResult, DbError> {
    let mut paths: Vec<String> = relative_paths
        .iter()
        .filter(|path| !path.is_empty())
        .cloned()
        .collect();
    paths.sort_unstable();
    paths.dedup();
    if paths.is_empty() {
        return Ok(ReconcileRemovalResult::default());
    }

    // A directory whose name ends in a media extension is rare; the exact update still catches it.
    let dir_paths: Vec<String> = paths
        .iter()
        .filter(|path| !settings.is_media_file(Path::new(path)))
        .cloned()
        .collect();

    let mut tx = pool.begin().await?;
    let cancelled = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled'
        WHERE status IN ('queued', 'running')
          AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')
          AND (
              relative_path = ANY($1)
              OR EXISTS (
                  SELECT 1
                  FROM unnest($2::text[]) AS p(prefix)
                  WHERE left(jobs.relative_path, length(p.prefix) + 1) = p.prefix || '/'
              )
          )
        "#,
        &paths,
        &dir_paths
    )
    .execute(&mut *tx)
    .await?;

    let marked = sqlx::query!(
        r#"
        UPDATE media_item
        SET missing_since = now(),
            updated_at = now()
        WHERE deleted = false
          AND missing_since IS NULL
          AND (
              relative_path = ANY($1)
              OR EXISTS (
                  SELECT 1
                  FROM unnest($2::text[]) AS p(prefix)
                  WHERE left(media_item.relative_path, length(p.prefix) + 1) = p.prefix || '/'
              )
          )
        "#,
        &paths,
        &dir_paths
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        DELETE FROM failed_ingest
        WHERE relative_path = ANY($1)
          OR EXISTS (
              SELECT 1
              FROM unnest($2::text[]) AS p(prefix)
              WHERE left(failed_ingest.relative_path, length(p.prefix) + 1) = p.prefix || '/'
          )
        "#,
        &paths,
        &dir_paths
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(ReconcileRemovalResult {
        marked_missing: marked.rows_affected() as usize,
        cancelled_jobs: cancelled.rows_affected(),
    })
}
