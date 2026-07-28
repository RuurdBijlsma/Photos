use api::serve;
use clap::Parser;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long, default_value_t = false, short, action)]
    no_scheduler: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let (settings, pool) = setup_binary().await?;
    serve(pool, settings, !Args::parse().no_scheduler).await?;

    Ok(())
}
