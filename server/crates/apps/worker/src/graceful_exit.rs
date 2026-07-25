use tokio::sync::watch::Receiver;
use tracing::{error, info};

#[must_use]
pub fn get_kill_signal() -> Receiver<bool> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
            let mut sigint =
                signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, initiating graceful shutdown...");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT (Ctrl-C), initiating graceful shutdown...");
                }
            }
        }
        #[cfg(not(unix))]
        {
            if let Err(e) = tokio::signal::ctrl_c().await {
                error!("Failed to register Ctrl-C handler: {}", e);
            } else {
                info!("Received Ctrl-C, initiating graceful shutdown...");
            }
        }
        let _ = shutdown_tx.send(true);
    });

    shutdown_rx
}
