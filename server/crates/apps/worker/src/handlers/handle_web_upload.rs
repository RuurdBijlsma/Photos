use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::require_media_mounted;
use crate::handlers::takeout_import::TakeoutImportPayload;
use app_state::MakeRelativePath;
use app_state::constants::{TAKEOUT_FOLDER, TUS_UPLOADS_FOLDER, USER_UPLOAD_FOLDER};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::jobs::{Job, JobType};
use common_services::database::user_store::UserStore;
use common_services::job_queue::{enqueue_full_ingest, enqueue_job};
use fileloft_core::UploadInfo;
use regex::Regex;
use sanitize_filename::sanitize;
use serde_json::from_value;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
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
    if let Some(result) =
        require_media_mounted(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }
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

    // Google Takeout exports arrive as chunked zips uploaded through the normal
    // upload dropzone. They are not media: stage them and hand them to the
    // takeout import pipeline instead of the file ingest.
    if is_takeout_zip(&sanitized_filename) {
        return stage_takeout_zip(context, user_id, payload.id.as_str(), &sanitized_filename).await;
    }

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

    let file_exists_at_destination = tokio::fs::try_exists(&destination_path)
        .await
        .unwrap_or(false);
    let file_exists_at_source = tokio::fs::try_exists(&uploaded_file).await.unwrap_or(false);

    if !file_exists_at_source && file_exists_at_destination {
        tracing::info!(
            "Source upload file is gone but destination file already exists at {:?}. Assuming already moved.",
            destination_path
        );
    } else {
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
    }

    // Attempt to clean up the metadata file, but ignore failures if it was already deleted
    if let Err(e) = fs::remove_file(&uploaded_metadata_file).await
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            "Failed to clean up metadata file {:?}: {}",
            uploaded_metadata_file,
            e
        );
    }

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

/// Recognizes a Google Takeout zip by its filename (`takeout-*.zip`).
fn is_takeout_zip(filename: &str) -> bool {
    let lower = filename.to_ascii_lowercase();
    lower.starts_with("takeout") && lower.rsplit_once('.').is_some_and(|(_, ext)| ext == "zip")
}

static TAKEOUT_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Matches "takeout-<8 digits>T<6 digits>Z-" case-insensitively
    Regex::new(r"(?i)^takeout-(\d{8}t\d{6}z)-").expect("Regex didn't compile")
});

/// Parses the takeout batch id (the export timestamp) from a filename like
/// `takeout-20261015T162830Z-1-001.zip`.
#[must_use]
pub fn takeout_batch_id(filename: &str) -> String {
    TAKEOUT_RE
        .captures(filename)
        .and_then(|caps| caps.get(1))
        .map_or_else(|| "takeout".to_owned(), |m| m.as_str().to_ascii_uppercase())
}

/// Moves a finished takeout zip out of the tus upload area and enqueues the
/// import pipeline for it.
async fn stage_takeout_zip(
    context: &WorkerContext,
    user_id: i32,
    tus_upload_id: &str,
    sanitized_filename: &str,
) -> Result<JobResult> {
    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);
    let uploaded_file = tus_dir.join(tus_upload_id);
    let uploaded_metadata_file = tus_dir.join(format!("{tus_upload_id}.info"));

    if !fs::try_exists(&uploaded_file).await.unwrap_or(false) {
        return Err(eyre!(
            "Takeout zip upload file is missing on disk: {tus_upload_id}"
        ));
    }

    let batch_id = takeout_batch_id(sanitized_filename);
    let staging_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TAKEOUT_FOLDER)
        .join(&batch_id);
    fs::create_dir_all(&staging_dir).await?;
    let staged_path = staging_dir.join(sanitized_filename);
    fs::rename(&uploaded_file, &staged_path).await?;
    // Best-effort metadata cleanup, same as the regular upload path.
    if let Err(e) = fs::remove_file(&uploaded_metadata_file).await
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(
            "Failed to clean up metadata file {:?}: {}",
            uploaded_metadata_file,
            e
        );
    }

    let staged_relative = staged_path.make_relative(&context.settings.ingest.app_data_root)?;
    enqueue_job(
        &context.pool,
        &context.settings.ingest,
        JobType::TakeoutImport,
    )
    .user_id(user_id)
    .payload(&TakeoutImportPayload {
        zip_relative_path: staged_relative,
        batch_id: batch_id.clone(),
    })
    .call()
    .await?;

    tracing::info!(
        "Staged takeout zip {:?} (batch {batch_id}) for import, user {user_id}",
        staged_path
    );
    Ok(JobResult::Done)
}
