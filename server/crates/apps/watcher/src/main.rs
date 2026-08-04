use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::graceful_exit::get_kill_signal;
use watcher::watcher::create_watcher;

#[tokio::main]
async fn main() -> Result<()> {
    let (settings, pool) = setup_binary().await?;
    let kill_signal = get_kill_signal();
    create_watcher(&pool, &settings.ingest, kill_signal).await?;

    Ok(())
}
