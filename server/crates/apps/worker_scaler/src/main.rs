use app_state::{database_url, load_app_settings};
use color_eyre::Result;
use common_services::database::get_db_pool;
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};
use worker_scaler::config::ScalerConfig;
use worker_scaler::start_scaler;

fn get_kill_signal() -> tokio::sync::watch::Receiver<bool> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => {}
                _ = sigint.recv() => {}
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        let _ = shutdown_tx.send(true);
    });

    shutdown_rx
}

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,ort=warn".into());
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let pool = get_db_pool(database_url(), false).await?;
    let config = ScalerConfig::default();
    let shutdown_rx = get_kill_signal();

    start_scaler(pool, settings, config, shutdown_rx).await?;

    Ok(())
}
