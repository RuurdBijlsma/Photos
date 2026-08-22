use crate::api::app_error::AppError;
use crate::api::jobs::interfaces::{
    IngestOverviewResponse, JobInfo, PaginatedJobsResponse, UserJobsQuery,
};
use crate::database::jobs::{JobStatus, JobType};
use sqlx::{PgPool, Postgres, QueryBuilder};

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

/// Retrieves a paginated list of user ingest jobs matching filter/search parameters.
pub async fn get_user_ingest_jobs(
    pool: &PgPool,
    user_id: i32,
    query: UserJobsQuery,
) -> Result<PaginatedJobsResponse, AppError> {
    let limit = query.limit.unwrap_or(10).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    // 1. Build and execute total count query
    let mut count_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM jobs WHERE user_id = ");
    count_builder.push_bind(user_id);
    count_builder.push(" AND job_type IN ('ingest_metadata'::job_type, 'ingest_thumbnails'::job_type, 'ingest_analysis'::job_type)");

    // Add status filter
    if let Some(status) = &query.status {
        match status.as_str() {
            "queued" => {
                count_builder.push(" AND status = 'queued'::job_status");
            }
            "running" | "processing" | "in_progress" => {
                count_builder.push(" AND status = 'running'::job_status");
            }
            "failed" => {
                count_builder.push(" AND status = 'failed'::job_status");
            }
            _ => {
                count_builder.push(" AND status IN ('queued'::job_status, 'running'::job_status, 'failed'::job_status)");
            }
        }
    } else {
        count_builder.push(
            " AND status IN ('queued'::job_status, 'running'::job_status, 'failed'::job_status)",
        );
    }

    if let Some(search) = &query.search
        && !search.trim().is_empty()
    {
        count_builder.push(" AND relative_path ILIKE ");
        count_builder.push_bind(format!("%{}%", search.trim()));
    }

    let count_query = count_builder.build_query_scalar::<i64>();
    let total = count_query.fetch_one(pool).await?;

    // 2. Build select query
    let mut select_builder = QueryBuilder::<Postgres>::new(
        "SELECT id, relative_path, user_id, job_type, payload, priority, status, attempts, \
         dependency_attempts, max_attempts, owner, started_at, finished_at, created_at, \
         scheduled_at, last_heartbeat, last_error FROM jobs WHERE user_id = ",
    );
    select_builder.push_bind(user_id);
    select_builder.push(" AND job_type IN ('ingest_metadata'::job_type, 'ingest_thumbnails'::job_type, 'ingest_analysis'::job_type)");

    // Add status filter
    if let Some(status) = &query.status {
        match status.as_str() {
            "queued" => {
                select_builder.push(" AND status = 'queued'::job_status");
            }
            "running" | "processing" | "in_progress" => {
                select_builder.push(" AND status = 'running'::job_status");
            }
            "failed" => {
                select_builder.push(" AND status = 'failed'::job_status");
            }
            _ => {
                select_builder.push(" AND status IN ('queued'::job_status, 'running'::job_status, 'failed'::job_status)");
            }
        }
    } else {
        select_builder.push(
            " AND status IN ('queued'::job_status, 'running'::job_status, 'failed'::job_status)",
        );
    }

    if let Some(search) = &query.search
        && !search.trim().is_empty()
    {
        select_builder.push(" AND relative_path ILIKE ");
        select_builder.push_bind(format!("%{}%", search.trim()));
    }

    select_builder.push(" ORDER BY id DESC LIMIT ");
    select_builder.push_bind(limit);
    select_builder.push(" OFFSET ");
    select_builder.push_bind(offset);

    let select_query = select_builder.build_query_as::<JobInfo>();
    let data = select_query.fetch_all(pool).await?;

    Ok(PaginatedJobsResponse {
        data,
        total,
        limit,
        offset,
    })
}
