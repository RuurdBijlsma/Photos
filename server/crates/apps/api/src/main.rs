use api::serve;
use clap::Parser;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::graceful_exit::get_kill_signal;
use model_provider::ModelProvider;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long, default_value_t = false, short, action)]
    no_scheduler: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let (settings, pool) = setup_binary().await?;
    let shutdown_rx = get_kill_signal();
    let model_provider = Arc::new(ModelProvider::new(
        settings.clone(),
        settings.scaler.unload_models_timeout,
    ));

    serve(
        pool,
        settings,
        model_provider,
        !Args::parse().no_scheduler,
        shutdown_rx,
    )
    .await?;

    Ok(())
}
