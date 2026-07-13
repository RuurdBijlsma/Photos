use crate::context::WorkerContext;
use crate::handlers::JobResult;
use app_state::MakeRelativePath;
use app_state::constants::{TUS_UPLOADS_FOLDER, USER_UPLOAD_FOLDER};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::jobs::Job;
use common_services::database::user_store::UserStore;
use common_services::job_queue::enqueue_full_ingest;
use fileloft_core::UploadInfo;
use sanitize_filename::sanitize;
use serde_json::from_value;
use std::path::{Path, PathBuf};
use tokio::fs;

async fn get_unique_filename(dest_folder: &Path, destination_path: &Path) -> PathBuf {
    let mut unique_destination_path = destination_path.to_path_buf();
    let mut counter = 1;

    while fs::try_exists(&unique_destination_path)
        .await
        .unwrap_or(false)
    {
        let stem = destination_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let extension = destination_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy();

        let new_filename = if extension.is_empty() {
            format!("{stem}_{counter}")
        } else {
            format!("{stem}_{counter}.{extension}")
        };

        unique_destination_path = dest_folder.join(new_filename);
        counter += 1;
    }

    unique_destination_path
}

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let user_id = job
        .user_id
        .ok_or_else(|| eyre!("HandleWebUpload job is missing a user_id"))?;
    let user_media_folder = UserStore::get_user_media_folder(&context.pool, user_id)
        .await?
        .ok_or_else(|| eyre!("HandleWebUpload: User has media folder set"))?;
    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("HandleWebUpload job is missing a payload"))?;
    let payload: UploadInfo = from_value(payload_value.clone())?;
    let user_provided_filename = payload
        .metadata
        .get("filename")
        .and_then(|opt| opt.as_ref())
        .ok_or_else(|| eyre!("HandleWebUpload payload metadata is missing filename"))?;
    let sanitized_filename = sanitize(user_provided_filename);

    dbg!(&payload);
    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);
    let uploaded_file = tus_dir.join(payload.id.as_str());
    let uploaded_metadata_file = tus_dir.join(format!("{}.info", payload.id.as_str()));
    let user_upload_folder = context
        .settings
        .ingest
        .media_root
        .join(user_media_folder)
        .join(USER_UPLOAD_FOLDER);
    let destination_path = get_unique_filename(
        &user_upload_folder,
        &user_upload_folder.join(&sanitized_filename),
    )
    .await;

    tokio::fs::create_dir_all(&user_upload_folder).await?;
    let temp_destination_path = get_unique_filename(
        &user_upload_folder,
        &user_upload_folder.join(format!("{sanitized_filename}.uploading")),
    )
    .await;

    if fs::rename(&uploaded_file, &destination_path).await.is_err() {
        fs::copy(&uploaded_file, &temp_destination_path).await?;
        fs::rename(&temp_destination_path, &destination_path).await?;
        fs::remove_file(&uploaded_file).await?;
    }
    fs::remove_file(&uploaded_metadata_file).await?;

    enqueue_full_ingest(
        &context.pool,
        &context.settings.ingest,
        &destination_path.make_relative(&context.settings.ingest.media_root)?,
        user_id,
        None,
    )
    .await?;

    Ok(JobResult::Done)
}
