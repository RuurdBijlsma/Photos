use clap::Parser;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::graceful_exit::get_kill_signal;
use model_provider::ModelProvider;
use photos_app::{AppOptions, run_app};
use std::sync::Arc;

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    let options = AppOptions::parse();
    let (settings, pool) = setup_binary().await?;
    let shutdown_rx = get_kill_signal();

    let model_provider = Arc::new(ModelProvider::new(
        settings.clone(),
        settings.scaler.unload_models_timeout,
    ));

    run_app(pool, settings, model_provider, options, shutdown_rx).await
}