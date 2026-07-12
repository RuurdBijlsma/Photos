use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::jobs::Job;
use common_types::ImportAlbumItemPayload;
use serde_json::from_value;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("ImportAlbumItem job is missing a payload"))?;
    let payload: ImportAlbumItemPayload = from_value(payload_value.clone())?;

    dbg!(&payload);
    Ok(JobResult::Done)
}
