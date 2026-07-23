use crate::context::WorkerContext;
use crate::graceful_exit::get_kill_signal;
use crate::handlers::handle_job;
use crate::jobs::management::{claim_next_job, update_job_on_completion, update_job_on_failure};
use app_state::AppSettings;
use color_eyre::Result;
use common_services::database::jobs::JobType;
use common_services::utils::nice_id;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{info};

#[allow(clippy::large_futures)]
pub async fn create_worker(
    pool: PgPool,
    settings: AppSettings,
    excluded_job_types: Vec<JobType>,
    stop_on_sleep: bool,
) -> Result<()> {
    let shutdown_rx = get_kill_signal();
    create_worker_with_shutdown(pool, settings, excluded_job_types, stop_on_sleep, shutdown_rx).await
}

#[allow(clippy::large_futures)]
pub async fn create_worker_with_shutdown(
    pool: PgPool,
    settings: AppSettings,
    excluded_job_types: Vec<JobType>,
    stop_on_sleep: bool,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let worker_id = nice_id(8);
    info!(
        "🛠️ [Worker ID: {}, IgnoreJobs: {:?}] Starting...",
        worker_id, excluded_job_types
    );
    let context = WorkerContext::new(pool, settings, worker_id.clone(), excluded_job_types).await?;

    run_worker_loop(&context, stop_on_sleep, shutdown_rx).await
}

/// The main loop for the worker process, continuously fetching and processing jobs.
///
/// # Errors
///
/// This function will return an error if there is a problem communicating with the
/// database when claiming or updating a job. The loop will terminate in such a case.
pub async fn run_worker_loop(
    context: &WorkerContext,
    stop_on_sleep: bool,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let mut sleeping = false;

    loop {
        // Check if shutdown has been requested before claiming a new job
        if *shutdown_rx.borrow() {
            info!("Shutdown requested. Exiting worker loop.");
            break;
        }

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

            // Sleep for 3 seconds or wake up early if a shutdown signal is received
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(3)) => {}
                _ = shutdown_rx.changed() => {
                    info!("Shutdown signal received during sleep. Exiting worker loop.");
                    break;
                }
            }
        }
    }
    Ok(())
}
