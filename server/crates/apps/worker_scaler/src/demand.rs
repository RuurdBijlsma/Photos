use app_state::constants::WORKER_HEARTBEAT_SECONDS;
use common_services::database::jobs::JobType;
use sqlx::PgPool;
use std::collections::HashMap;

/// Queries the database for all currently claimable jobs and returns a mapping
/// of `JobType` to available job count.
pub async fn get_demand(pool: &PgPool) -> color_eyre::Result<HashMap<JobType, usize>> {
    struct JobCount {
        pub job_type: JobType,
        pub count: i64,
    }

    let available_jobs_with_count: HashMap<JobType, usize> = sqlx::query_as!(
        JobCount,
        r#"
        WITH active_jobs AS (
            SELECT relative_path, job_type, scope, phase, status, last_heartbeat
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
        )
        SELECT
            j.job_type::text AS "job_type!: JobType",
            COUNT(*)::bigint AS "count!"
        FROM active_jobs j
        CROSS JOIN phase_limits pl
        LEFT JOIN path_limits pl_path ON j.relative_path = pl_path.relative_path
        WHERE
            -- Claimable check: queued OR stale running
            (j.status = 'queued' OR j.last_heartbeat < now() - interval '1 second' * $1)

            -- Global phase check: no active job can have lower phase than a global job
            AND (j.scope != 'global' OR j.phase <= pl.min_all_phase)

            -- Path phase check: path job cannot be blocked by global or same-path lower phase jobs
            AND (j.scope != 'path' OR (
                (pl.min_global_phase IS NULL OR j.phase <= pl.min_global_phase)
                AND (j.relative_path IS NULL OR j.phase <= pl_path.min_path_phase)
            ))
        GROUP BY j.job_type
        "#,
        WORKER_HEARTBEAT_SECONDS
    )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|jc| (jc.job_type, jc.count as usize))
        .collect();

    Ok(available_jobs_with_count)
}