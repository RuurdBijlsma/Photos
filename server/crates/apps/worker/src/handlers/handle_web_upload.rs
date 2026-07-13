use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use common_services::database::jobs::Job;
use fileloft_core::UploadInfo;
use serde_json::from_value;
use tokio::fs;
use app_state::constants::TUS_UPLOADS_FOLDER;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("HandleWebUpload job is missing a payload"))?;
    let payload: UploadInfo = from_value(payload_value.clone())?;
    let tus_dir = context.settings.ingest.app_data_root.join(TUS_UPLOADS_FOLDER);
    let mut entries = fs::read_dir(&tus_dir).await?;

    println!("Contents of {}:", tus_dir.display());

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;

        if metadata.is_file() {
            println!("{} ({} bytes)", path.display(), metadata.len());
        } else {
            println!("{}/", path.display());
        }
    }
    // todo: also other stuff like use rename to get it in the media folder, first move to file.mp4.tmp or something

    dbg!(&payload);
    Ok(JobResult::Done)
}
