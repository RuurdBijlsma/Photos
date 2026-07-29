use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::macros::backoff_seconds;
use app_state::constants::WORKER_HEARTBEAT_SECONDS;
use chrono::{Duration, Utc};
use color_eyre::{Report, Result};
use common_services::alert;
use common_services::database::jobs::{Job, JobScope, JobStatus, JobType};
use sqlx::{Executor, PgPool, Postgres};
use tracing::{info, warn};

/// Atomically claims the next available job from the queue based on phase and scope ordering.
pub async fn claim_next_job(context: &WorkerContext) -> Result<Option<Job>> {
    let mut tx = context.pool.begin().await?;

    let excluded_strings: Vec<String> = context
        .excluded_job_types
        .iter()
        .filter_map(|jt| {
            serde_json::to_value(jt)
                .ok()
                .and_then(|v| v.as_str().map(ToString::to_string))
        })
        .collect();

    let job = sqlx::query_as!(
        Job,
        r#"
        WITH active_jobs AS (
            SELECT relative_path, scope, phase
            FROM jobs
            WHERE status = 'running' OR (status = 'queued' AND scheduled_at <= now())
        ),
        phase_limits AS (
            SELECT
                MIN(phase) AS min_all_phase,
                MIN(phase) FILTER (WHERE scope = 'global') AS min_global_phase
            FROM active_jobs
        ),
        path_limits AS (
            SELECT relative_path, MIN(phase) AS min_path_phase
            FROM active_jobs
            WHERE relative_path IS NOT NULL
            GROUP BY relative_path
        ),
        candidate AS (
            SELECT j.id
            FROM jobs j
            CROSS JOIN phase_limits pl
            LEFT JOIN path_limits pl_path ON j.relative_path = pl_path.relative_path
            WHERE
                ((j.status = 'queued' AND j.scheduled_at <= now())
                OR (j.status = 'running' AND j.last_heartbeat < now() - interval '1 second' * $2))
              AND j.job_type::text != ALL($3::text[])

              -- Global phase check
              AND (j.scope != 'global' OR j.phase <= pl.min_all_phase)

              -- Path phase check
              AND (j.scope != 'path' OR (
                  (pl.min_global_phase IS NULL OR j.phase <= pl.min_global_phase)
                  AND (j.relative_path IS NULL OR j.phase <= pl_path.min_path_phase)
              ))
            ORDER BY j.priority ASC, j.relative_path DESC, j.scheduled_at ASC, j.created_at ASC
            FOR UPDATE OF j SKIP LOCKED
            LIMIT 1
        )
        UPDATE jobs
        SET status = 'running',
            owner = $1,
            started_at = now(),
            last_heartbeat = now(),
            attempts = CASE WHEN jobs.status = 'running' THEN jobs.attempts + 1 ELSE jobs.attempts END
        FROM candidate
        WHERE jobs.id = candidate.id
        RETURNING jobs.id,
                  jobs.payload,
                  jobs.relative_path,
                  jobs.job_type AS "job_type!: JobType",
                  jobs.scope AS "scope!: JobScope",
                  jobs.phase,
                  jobs.priority,
                  jobs.user_id,
                  jobs.attempts,
                  jobs.max_attempts,
                  jobs.dependency_attempts
        "#,
        context.worker_id,
        WORKER_HEARTBEAT_SECONDS,
        &excluded_strings as &[String]
    )
        .fetch_optional(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(job)
}

/// Updates a job's status based on a successful completion result.
pub async fn update_job_on_completion(pool: &PgPool, job: &Job, result: JobResult) -> Result<()> {
    match result {
        JobResult::Done => mark_job_done(pool, job.id).await,
        JobResult::Cancelled => mark_job_cancelled(pool, job.id).await,
        JobResult::DependencyReschedule => {
            if job.dependency_attempts > 10 {
                alert!(
                    "Alarmingly many attempts to dependency reschedule job {}.",
                    job.id
                );
            }
            let delay = backoff_seconds(job.dependency_attempts);
            dependency_reschedule_job(pool, job.id, delay).await
        }
    }
}

/// Updates a job's status on failure, either marking it as failed or rescheduling it.
pub async fn update_job_on_failure(pool: &PgPool, job: &Job, error: &Report) -> Result<()> {
    let error_string = &format!("{error:?}");
    if job.attempts + 1 >= job.max_attempts {
        mark_job_failed(pool, job.id, error_string).await
    } else {
        let delay = backoff_seconds(job.attempts);
        reschedule_for_retry(pool, job.id, delay, error_string).await
    }
}

/// Marks a job as done in the database.
async fn mark_job_done(pool: &PgPool, job_id: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE jobs SET status = 'done', finished_at = now() WHERE id = $1",
        job_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Marks a job as cancelled in the database.
async fn mark_job_cancelled(pool: &PgPool, job_id: i64) -> Result<()> {
    sqlx::query!("UPDATE jobs SET status = 'cancelled' WHERE id = $1", job_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Marks a job as failed in the database.
async fn mark_job_failed(pool: &PgPool, job_id: i64, last_error: &str) -> Result<()> {
    alert!("🚨 Marking job {} as failed: {}", job_id, last_error);
    sqlx::query!(
        "UPDATE jobs SET status = 'failed', finished_at = now(), last_error = $2, attempts = attempts + 1 WHERE id = $1",
        job_id,
        last_error
    )
        .execute(pool)
        .await?;
    Ok(())
}

/// Reschedules a job to be tried again after a backoff period.
async fn reschedule_for_retry(
    pool: &PgPool,
    job_id: i64,
    backoff_secs: i64,
    last_error: &str,
) -> Result<()> {
    warn!("⚠️ Rescheduling job {}. Backoff: {}s", job_id, backoff_secs);
    println!("{last_error}");
    let scheduled_at = Utc::now() + Duration::seconds(backoff_secs);
    sqlx::query!(
        "UPDATE jobs SET status = 'queued', scheduled_at = $2, attempts = attempts + 1, owner = NULL, started_at = NULL, last_error = $3 WHERE id = $1",
        job_id,
        scheduled_at,
        last_error
    )
        .execute(pool)
        .await?;
    Ok(())
}

/// Reschedules a job because its dependencies are not met.
async fn dependency_reschedule_job(pool: &PgPool, job_id: i64, backoff_secs: i64) -> Result<()> {
    info!(
        "⏳ Dependency not met for job {}. Rescheduling in {}s.",
        job_id, backoff_secs
    );
    let scheduled_at = Utc::now() + Duration::seconds(backoff_secs);
    sqlx::query!(
        "UPDATE jobs SET status = 'queued', scheduled_at = $2, dependency_attempts = dependency_attempts + 1, owner = NULL, started_at = NULL, last_error = NULL WHERE id = $1",
        job_id,
        scheduled_at
    )
        .execute(pool)
        .await?;
    Ok(())
}

/// Checks if a job has been cancelled within a given transaction.
pub async fn is_job_cancelled(
    executor: impl Executor<'_, Database = Postgres>,
    job_id: i64,
) -> Result<bool> {
    let status: Option<JobStatus> = sqlx::query_scalar("SELECT status FROM jobs WHERE id = $1")
        .bind(job_id)
        .fetch_optional(executor)
        .await?;
    Ok(status.is_none_or(|s| s == JobStatus::Cancelled))
}
