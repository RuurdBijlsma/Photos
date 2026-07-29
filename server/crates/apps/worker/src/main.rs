use clap::Parser;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::database::jobs::JobType;
use worker::worker::create_worker;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(long, short, value_delimiter = ',', value_parser = parse_job_type)]
    exclude: Vec<JobType>,
}

fn parse_job_type(s: &str) -> Result<JobType, String> {
    serde_json::from_str::<JobType>(&format!("\"{s}\""))
        .map_err(|e| format!("Invalid job type '{s}': {e}"))
}

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> Result<()> {
    let (settings, pool) = setup_binary().await?;
    create_worker(pool, settings, Args::parse().exclude, false).await?;

    Ok(())
}
