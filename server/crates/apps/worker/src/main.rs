use clap::Parser;
use color_eyre::Result;
use common_services::binary_setup::setup_binary;
use common_services::database::jobs::JobType;
use common_services::graceful_exit::get_kill_signal;
use common_services::utils::nice_id;
use worker::models::WorkerModels;
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
    let kill_signal = get_kill_signal();
    let excluded_jobs = Args::parse().exclude;
    let models = WorkerModels::load_for_jobs(&settings, &excluded_jobs).await?;

    create_worker(
        pool,
        settings,
        nice_id(8),
        excluded_jobs,
        models,
        false, // keep running
        kill_signal,
    )
        .await?;

    Ok(())
}
