use app_state::AppSettings;
use app_state::constants::WORKER_HEARTBEAT_SECONDS;
use common_services::database::jobs::JobType;
use sqlx::PgPool;
use std::collections::HashMap;

pub async fn get_demand(
    pool: &PgPool,
    settings: &AppSettings,
) -> color_eyre::Result<HashMap<String, usize>> {
    struct JobCount {
        pub job_type: JobType,
        pub count: i64,
    }

    let available_jobs_with_count: HashMap<JobType, usize> = sqlx::query_as!(JobCount, r#"
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

    let mut profile_demand: HashMap<String, usize> = HashMap::new();
    for profile in &settings.scaler.profiles {
        profile_demand.insert(profile.name.clone(), 0);
    }

    // Dynamic routing: map queued jobs to the cheapest capable profile configured.
    for (job_type, count) in available_jobs_with_count {
        if let Some(best_fit_profile) = settings
            .scaler
            .profiles
            .iter()
            .filter(|profile| {
                !profile.excluded_jobs.iter().any(|excluded| {
                    JobType::parse_from_str(excluded)
                        .is_ok_and(|excluded_type| excluded_type == job_type)
                })
            })
            .min_by_key(|profile| profile.estimated_ram_mb)
            && let Some(demand) = profile_demand.get_mut(&best_fit_profile.name)
        {
            *demand += count;
        }
    }

    Ok(profile_demand)
}
