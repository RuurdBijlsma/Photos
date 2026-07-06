use api::serve;
use app_state::{database_url, load_app_settings};
use clap::Parser;
use color_eyre::Result;
use common_services::database::get_db_pool;
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long, default_value_t = false, short, action)]
    no_scheduler: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,ort=warn,api=debug,tower_http=debug".into());
    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let pool = get_db_pool(database_url(), true).await?;
    serve(pool, settings, !Args::parse().no_scheduler).await?;

    Ok(())
}
