use crate::context::WorkerContext;
use crate::handlers::handle_job;
use crate::jobs::management::{claim_next_job, update_job_on_completion, update_job_on_failure};
use app_state::AppSettings;
use color_eyre::Result;
use common_services::utils::nice_id;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

pub async fn create_worker(
    pool: PgPool,
    settings: AppSettings,
    handle_llm: bool,
    stop_on_sleep: bool,
) -> Result<()> {
    let worker_id = nice_id(8);
    info!("🛠️ [Worker ID: {}] Starting.", worker_id);
    let context = WorkerContext::new(pool, settings, worker_id.clone(), handle_llm).await?;

    run_worker_loop(&context, stop_on_sleep).await
}

/// The main loop for the worker process, continuously fetching and processing jobs.
///
/// # Errors
///
/// This function will return an error if there is a problem communicating with the
/// database when claiming or updating a job. The loop will terminate in such a case.
pub async fn run_worker_loop(context: &WorkerContext, stop_on_sleep: bool) -> Result<()> {
    let mut sleeping = false;

    loop {
        let maybe_job = claim_next_job(context).await?;

        if let Some(job) = maybe_job {
            sleeping = false;
            info!(
                "🐜 Picked up {:?} job: {:?}",
                job.job_type, job.relative_path
            );

            let job_result = handle_job(context, &job).await;

            match job_result {
                Ok(result) => update_job_on_completion(&context.pool, &job, result).await?,
                Err(e) => update_job_on_failure(&context.pool, &job, &e).await?,
            }
        } else {
            if !sleeping {
                sleeping = true;
                info!("💤 No jobs, going to sleep...");
                if stop_on_sleep {
                    return Ok(());
                }
            }
            sleep(Duration::from_secs(3)).await;
        }
    }
}
