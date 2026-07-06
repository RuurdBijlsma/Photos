use crate::context::WorkerContext;
use crate::handlers::JobResult;
use chrono::Utc;
use color_eyre::Result;
use common_services::database::jobs::{Job, JobType};
use common_services::job_queue::enqueue_job;

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    enqueue_job::<()>(&context.pool, &context.settings.ingest, JobType::Scan)
        .scheduled_at(Utc::now() + chrono::Duration::minutes(1))
        .call()
        .await?;

    Ok(JobResult::Done)
}
