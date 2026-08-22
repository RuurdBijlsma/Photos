use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::graceful_exit::get_kill_signal;
use model_provider::ModelProvider;
use std::sync::Arc;
use worker_scaler::start_scaler;

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    let (settings, pool) = setup_binary().await?;
    let shutdown_rx = get_kill_signal();
    let model_provider = Arc::new(ModelProvider::new(
        settings.clone(),
        settings.scaler.unload_models_timeout,
    ));

    start_scaler(pool, settings, model_provider, shutdown_rx).await?;

    Ok(())
}