use common_services::database::jobs::JobType;
use sqlx::{PgPool, Row};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct QueueDemand {
    pub light_demand: usize,
    pub medium_demand: usize,
    pub heavy_demand: usize,
    pub llm_demand: usize,
}

pub async fn query_queue_demand(pool: &PgPool) -> color_eyre::Result<QueueDemand> {
    let heartbeat_timeout_seconds = 150.0;

    let rows = sqlx::query(
        r"
        SELECT j.job_type::text AS job_type, COUNT(*)::bigint AS count
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
        ",
    )
    .bind(heartbeat_timeout_seconds)
    .fetch_all(pool)
    .await?;

    let mut counts: HashMap<JobType, usize> = HashMap::new();
    for row in rows {
        let jt_str: String = row.get("job_type");
        let count: i64 = row.get("count");
        if let Ok(jt) = serde_json::from_str::<JobType>(&format!("\"{jt_str}\"")) {
            counts.insert(jt, usize::try_from(count).unwrap_or(0));
        }
    }

    let mut demand = QueueDemand::default();

    for (&jt, &cnt) in &counts {
        match jt {
            JobType::IngestLlm => {
                demand.llm_demand += cnt;
            }
            JobType::IngestThumbnails => {
                demand.heavy_demand += cnt;
            }
            JobType::IngestAnalysis | JobType::ClusterFaces | JobType::ClusterPhotos => {
                demand.medium_demand += cnt;
            }
            _ => {
                demand.light_demand += cnt;
            }
        }
    }

    Ok(demand)
}
