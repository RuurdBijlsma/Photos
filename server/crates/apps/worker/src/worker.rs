use crate::context::WorkerContext;
use crate::handlers::{JobResult, handle_job};
use crate::jobs::management::{claim_next_job, update_job_on_completion, update_job_on_failure};
use crate::models::WorkerModels;
use app_state::AppSettings;
use color_eyre::Result;
use common_services::alert;
use common_services::database::jobs::JobType;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{Instrument, info, info_span, warn};

#[allow(clippy::large_futures)]
pub async fn create_worker(
    pool: PgPool,
    settings: AppSettings,
    worker_id: String,
    excluded_job_types: Vec<JobType>,
    models: WorkerModels,
    stop_on_sleep: bool,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let worker_span = info_span!("", id = %worker_id);
    info!(
        "🛠️ [Worker ID: {}, IgnoreJobs: {:?}] Starting...",
        worker_id, excluded_job_types
    );
    let context = WorkerContext::new(
        pool,
        settings,
        worker_id.clone(),
        excluded_job_types,
        models,
    );
    run_worker_loop(&context, stop_on_sleep, shutdown_rx)
        .instrument(worker_span)
        .await
}

/// The main loop for the worker process, continuously fetching and processing jobs.
pub async fn run_worker_loop(
    context: &WorkerContext,
    stop_on_sleep: bool,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let mut sleepiness: u32 = 0;

    loop {
        if *shutdown_rx.borrow() {
            info!("Shutdown requested. Exiting worker loop.");
            break;
        }

        let maybe_job = claim_next_job(context).await?;

        if let Some(job) = maybe_job {
            sleepiness = 0;
            info!(
                "🐜 Picked up {:?} job: {:?}",
                job.job_type, job.relative_path
            );

            let job_result = handle_job(context, &job).await;

            let storage_unavailable = matches!(&job_result, Ok(JobResult::StorageUnavailable(_)));

            match job_result {
                Ok(result) => update_job_on_completion(&context.pool, &job, result).await?,
                Err(e) => update_job_on_failure(&context.pool, &job, &e).await?,
            }

            if storage_unavailable {
                alert!("Mounted folder is unavailable");
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(60)) => {}
                    _ = shutdown_rx.changed() => {
                        info!("Shutdown signal received while waiting for media folder. Exiting worker loop.");
                        break;
                    }
                }
            }
        } else {
            sleepiness += 1;
            if stop_on_sleep && sleepiness >= 2 {
                info!("💤 No jobs, going to sleep...");
                return Ok(());
            }

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
