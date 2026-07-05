use sqlx::PgPool;
use tokio::task::JoinHandle;

/// Spawns a background task to periodically update the `last_heartbeat` for a running job.
#[must_use]
pub fn start_heartbeat_loop(pool: &PgPool, job_id: i64) -> JoinHandle<()> {
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_mins(1));
        loop {
            interval.tick().await;
            let result = sqlx::query!(
                "UPDATE jobs SET last_heartbeat = now() WHERE id = $1 AND status = 'running'",
                job_id
            )
            .execute(&pool_clone)
            .await;

            let Ok(res) = result else { break };
            if res.rows_affected() == 0 {
                break;
            }
        }
    })
}
