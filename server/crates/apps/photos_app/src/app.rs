use app_state::AppSettings;
use clap::Parser;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use model_provider::ModelProvider;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::info;

#[derive(Parser, Debug, Clone, Default)]
#[command(version, about = "Ruurd Photos Server", long_about = None)]
#[allow(clippy::struct_excessive_bools)]
pub struct AppOptions {
    /// Disable the API web server
    #[clap(long, default_value_t = false)]
    pub no_api: bool,

    /// Disable the background worker scaler
    #[clap(long, default_value_t = false)]
    pub no_scaler: bool,

    /// Disable the file system watcher
    #[clap(long, default_value_t = false)]
    pub no_watcher: bool,

    /// Disable the background cron/task scheduler in the API
    #[clap(long, default_value_t = false)]
    pub no_scheduler: bool,
}

#[allow(clippy::large_futures)]
pub async fn run_app(
    pool: PgPool,
    settings: AppSettings,
    model_provider: Arc<ModelProvider>,
    options: AppOptions,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    info!("🏃‍♂️➡️ Starting Ruurd Photos...");

    let mut handles: Vec<JoinHandle<Result<()>>> = Vec::new();

    // 1. API Server & Task Scheduler
    if options.no_api {
        info!("ℹ️ API server is disabled by --no-api");
    } else {
        let pool = pool.clone();
        let settings = settings.clone();
        let model_provider = model_provider.clone();
        let shutdown_rx = shutdown_rx.clone();
        let run_task_scheduler = !options.no_scheduler;

        handles.push(tokio::spawn(async move {
            api::serve(
                pool,
                settings,
                model_provider,
                run_task_scheduler,
                shutdown_rx,
            )
                .await
                .map_err(|e| eyre!("API server failed: {e}"))
        }));
    }

    // 2. Worker Scaler & Queue Consumer
    if options.no_scaler {
        info!("ℹ️ Worker scaler is disabled by --no-scaler");
    } else {
        let pool = pool.clone();
        let settings = settings.clone();
        let model_provider = model_provider.clone();
        let shutdown_rx = shutdown_rx.clone();

        handles.push(tokio::spawn(async move {
            worker_scaler::start_scaler(pool, settings, model_provider, shutdown_rx)
                .await
                .map_err(|e| eyre!("Worker scaler failed: {e}"))
        }));
    }

    // 3. File System Watcher
    if options.no_watcher {
        info!("ℹ️ File watcher is disabled by --no-watcher");
    } else {
        let pool = pool.clone();
        let ingest_settings = settings.ingest.clone();
        let shutdown_rx = shutdown_rx.clone();

        handles.push(tokio::spawn(async move {
            watcher::watcher::create_watcher(&pool, &ingest_settings, shutdown_rx)
                .await
                .map_err(|e| eyre!("File watcher failed: {e}"))
        }));
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