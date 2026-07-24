use app_state::AppSettings;
use app_state::constants::WORKER_HEARTBEAT_SECONDS;
use common_services::database::jobs::JobType;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

pub async fn query_queue_demand(
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
          AND (
              j.relative_path IS NULL
              OR NOT EXISTS (
                  SELECT 1 FROM jobs dep
                  WHERE dep.relative_path = j.relative_path
                    AND dep.status != 'done'
                    AND (
                        (j.job_type = 'ingest_thumbnails' AND dep.job_type = 'ingest_metadata')
                        OR (j.job_type IN ('ingest_analysis', 'ingest_llm') AND dep.job_type IN ('ingest_metadata', 'ingest_thumbnails'))
                    )
              )
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
    for profile_name in settings.scaler.profiles.keys() {
        profile_demand.insert(profile_name.clone(), 0);
    }

    // Dynamic routing: map queued jobs to the cheapest capable profile configured
    for (job_type, count) in available_jobs_with_count {
        let mut capable_profiles: Vec<(&String, &app_state::ProfileSettings)> = settings
            .scaler
            .profiles
            .iter()
            .filter(|(_, p_settings)| {
                let is_excluded = p_settings.excluded_jobs.iter().any(|ex_job| {
                    if let Ok(ex_jt) = serde_json::from_str::<JobType>(&format!("\"{ex_job}\"")) {
                        ex_jt == job_type
                    } else {
                        false
                    }
                });
                !is_excluded
            })
            .collect();

        // Sort ascending by RAM footprint to select the cheapest compatible worker
        capable_profiles.sort_by_key(|(_, p_settings)| p_settings.estimated_ram_mb);

        if let Some((cheapest_profile, _)) = capable_profiles.first() {
            if let Some(demand_value) = profile_demand.get_mut(*cheapest_profile) {
                *demand_value += count;
            }
        }
    }

    Ok(profile_demand)
}
