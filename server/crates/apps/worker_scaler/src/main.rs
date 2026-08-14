use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::graceful_exit::get_kill_signal;
use worker_scaler::start_scaler;
use ort::environment::GlobalThreadPoolOptions;

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    ort::init()
        .with_global_thread_pool(
            GlobalThreadPoolOptions::default()
                .with_intra_threads(2)?
                .with_inter_threads(2)?,
        )
        .commit();

    let (settings, pool) = setup_binary().await?;
    let shutdown_rx = get_kill_signal();

    start_scaler(pool, settings, shutdown_rx).await?;

    Ok(())
}
