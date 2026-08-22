use clap::Parser;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::graceful_exit::get_kill_signal;
use model_provider::ModelProvider;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about = "Ruurd Photos Monolith Server", long_about = None)]
struct Args {
    /// Disable the API web server
    #[clap(long, default_value_t = false)]
    no_api: bool,

    /// Disable the background worker scaler
    #[clap(long, default_value_t = false)]
    no_scaler: bool,

    /// Disable the file system watcher
    #[clap(long, default_value_t = false)]
    no_watcher: bool,

    /// Disable the background cron/task scheduler in the API
    #[clap(long, default_value_t = false)]
    no_scheduler: bool,
}

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    let args = Args::parse();
    let (settings, pool) = setup_binary().await?;
    let shutdown_rx = get_kill_signal();

    let model_provider = Arc::new(ModelProvider::new(
        settings.clone(),
        settings.scaler.unload_models_timeout,
    ));

    info!("🏃‍♂️‍➡️ Starting Ruurd Photos...");

    let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

    // 1. API Server & Task Scheduler
    if !args.no_api {
        let pool = pool.clone();
        let settings = settings.clone();
        let model_provider = model_provider.clone();
        let shutdown_rx = shutdown_rx.clone();
        let run_task_scheduler = !args.no_scheduler;

        handles.push(tokio::spawn(async move {
            api::serve(
                pool,
                settings,
                model_provider,
                run_task_scheduler,
                shutdown_rx,
            )
                .await
                .map_err(|e| color_eyre::eyre::eyre!("API server failed: {e}"))
        }));
    } else {
        info!("ℹ️ API server is disabled by --no-api");
    }

    // 2. Worker Scaler & Queue Consumer
    if !args.no_scaler {
        let pool = pool.clone();
        let settings = settings.clone();
        let model_provider = model_provider.clone();
        let shutdown_rx = shutdown_rx.clone();

        handles.push(tokio::spawn(async move {
            worker_scaler::start_scaler(pool, settings, model_provider, shutdown_rx)
                .await
                .map_err(|e| color_eyre::eyre::eyre!("Worker scaler failed: {e}"))
        }));
    } else {
        info!("ℹ️ Worker scaler is disabled by --no-scaler");
    }

    // 3. File System Watcher
    if !args.no_watcher {
        let pool = pool.clone();
        let ingest_settings = settings.ingest.clone();
        let shutdown_rx = shutdown_rx.clone();

        handles.push(tokio::spawn(async move {
            watcher::watcher::create_watcher(&pool, &ingest_settings, shutdown_rx)
                .await
                .map_err(|e| color_eyre::eyre::eyre!("File watcher failed: {e}"))
        }));
    } else {
        info!("ℹ️ File watcher is disabled by --no-watcher");
    }

    // Await all subsystems until graceful termination
    for handle in handles {
        if let Err(e) = handle.await? {
            tracing::error!("{e}");
        }
    }

    info!("👋 Ruurd Photos shut down cleanly.");
    Ok(())
}