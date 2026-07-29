mod handlers;
mod watcher;

use crate::watcher::start_watching;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;

#[tokio::main]
async fn main() -> Result<()> {
    let (settings, pool) = setup_binary().await?;
    start_watching(&pool, &settings.ingest).await?;

    Ok(())
}
