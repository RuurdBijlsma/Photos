use crate::api::app_error::AppError;
use crate::api::jobs::interfaces::{IngestOverviewResponse, JobInfo};
use crate::database::jobs::{JobStatus, JobType};
use sqlx::PgPool;

/// Retrieves the counts of queued, running, failed, completed, and cancelled ingest jobs.
/// Leverages the cleanDB job routine to only count recently relevant records.
pub async fn get_user_ingest_overview(
    pool: &PgPool,
    user_id: i32,
) -> Result<IngestOverviewResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            job_type AS "job_type: JobType",
            status AS "status: JobStatus",
            COUNT(*)::bigint AS "count!"
        FROM jobs
        WHERE user_id = $1
          AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis', 'ingest_llm')
        GROUP BY job_type, status
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut overview = IngestOverviewResponse::default();

    for row in rows {
        let counts = match row.job_type {
            JobType::IngestMetadata => &mut overview.metadata,
            JobType::IngestThumbnails => &mut overview.thumbnails,
            JobType::IngestAnalysis => &mut overview.analysis,
            JobType::IngestLlm => &mut overview.llm,
            _ => continue,
        };

        let count = row.count;
        counts.total += count;

        match row.status {
            JobStatus::Queued => counts.queued = count,
            JobStatus::Running => counts.running = count,
            JobStatus::Failed => counts.failed = count,
            JobStatus::Done => counts.done = count,
            JobStatus::Cancelled => counts.cancelled = count,
        }
    }

    Ok(overview)
}

/// Lists all running ingest jobs for the current user.
///
/// This includes successfully completed within the last 5 seconds to feed a real-time activity
/// stream. Filters out jobs marked as 'running' if their heartbeat is older than 1 minute 30
/// seconds.
pub async fn get_running_ingest_jobs(
    pool: &PgPool,
    user_id: i32,
) -> Result<Vec<JobInfo>, AppError> {
    let jobs = sqlx::query_as!(
        JobInfo,
        r#"
        SELECT
            id,
            relative_path,
            user_id,
            job_type AS "job_type: JobType",
            payload,
            priority,
            status AS "status: JobStatus",
            attempts,
            dependency_attempts,
            max_attempts,
            owner,
            started_at,
            finished_at,
            created_at,
            scheduled_at,
            last_heartbeat,
            last_error
        FROM jobs
        WHERE user_id = $1
          AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis', 'ingest_llm')
          AND (
              (status = 'running'::job_status AND last_heartbeat >= NOW() - INTERVAL '1 minute 30 seconds')
              OR (status = 'done'::job_status AND finished_at >= NOW() - INTERVAL '5 seconds')
          )
        ORDER BY started_at DESC NULLS LAST, finished_at DESC NULLS LAST
        "#,
        user_id
    )
        .fetch_all(pool)
        .await?;

    Ok(jobs)
}

/// Lists failed ingest jobs for the current user.
pub async fn get_failed_ingest_jobs(pool: &PgPool, user_id: i32) -> Result<Vec<JobInfo>, AppError> {
    let jobs = sqlx::query_as!(
        JobInfo,
        r#"
        SELECT
            id,
            relative_path,
            user_id,
            job_type AS "job_type: JobType",
            payload,
            priority,
            status AS "status: JobStatus",
            attempts,
            dependency_attempts,
            max_attempts,
            owner,
            started_at,
            finished_at,
            created_at,
            scheduled_at,
            last_heartbeat,
            last_error
        FROM jobs
        WHERE user_id = $1
          AND status = 'failed'::job_status
          AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis', 'ingest_llm')
        ORDER BY finished_at DESC NULLS LAST
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    Ok(jobs)
}

/// Triggers a scan by enqueuing a scanning job.
/// Utilizes ON CONFLICT DO NOTHING to deduplicate duplicate requests gracefully.
pub async fn enqueue_scan_job(pool: &PgPool, user_id: i32) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO jobs (job_type, user_id, status)
        VALUES ('scan'::job_type, $1, 'queued'::job_status)
        ON CONFLICT DO NOTHING
        "#,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Scopes retry requests safely to the authorized user's own ingestion pipeline.
pub async fn retry_user_job(pool: &PgPool, job_id: i64, user_id: i32) -> Result<(), AppError> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'queued'::job_status,
            attempts = 0,
            scheduled_at = NOW(),
            finished_at = NULL,
            started_at = NULL,
            last_error = NULL,
            owner = NULL
        WHERE id = $1
          AND user_id = $2
          AND status IN ('failed'::job_status, 'done'::job_status, 'cancelled'::job_status)
          AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis', 'ingest_llm')
        "#,
        job_id,
        user_id
    )
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest(
            "Job cannot be retried (it might not exist, belongs to another user, or is not in a retryable state)".to_owned(),
        ));
    }

    Ok(())
}
