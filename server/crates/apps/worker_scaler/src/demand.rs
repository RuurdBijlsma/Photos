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
        SELECT
            j.job_type::text AS "job_type!: JobType",
            COUNT(*)::bigint AS "count!"
        FROM jobs j
        WHERE
            ((j.status = 'queued' AND j.scheduled_at <= now())
            OR (j.status = 'running' AND j.last_heartbeat < now() - interval '1 second' * $1))

          -- Path-scoped phase check
          AND (
              j.relative_path IS NULL
              OR NOT EXISTS (
                  SELECT 1 FROM jobs dep
                  WHERE dep.relative_path = j.relative_path
                    AND dep.phase < j.phase
                    AND ((dep.status = 'queued' AND dep.scheduled_at <= now()) OR dep.status = 'running')
              )
          )

          -- Global-scoped: ANY lower-phase job blocks global jobs, or lower-phase global jobs block path jobs
          AND NOT EXISTS (
              SELECT 1 FROM jobs dep
              WHERE dep.phase < j.phase
                AND ((dep.status = 'queued' AND dep.scheduled_at <= now()) OR dep.status = 'running')
                AND (dep.scope = 'global' OR j.scope = 'global')
          )
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
