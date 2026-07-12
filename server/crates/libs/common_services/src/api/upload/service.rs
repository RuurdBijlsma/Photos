use crate::api::upload::interfaces::UploadCompletePayload;
use crate::database::user_store::UserStore;
use crate::job_queue::enqueue_full_ingest;
use app_state::IngestSettings;
use color_eyre::eyre::{eyre, Result};
use sqlx::PgPool;
use std::fs;
use tracing::info;

pub async fn handle_upload_complete(
    pool: &PgPool,
    settings: &IngestSettings,
    user_id: i32,
    payload: UploadCompletePayload,
) -> Result<()> {
    // 1. Resolve user folder configuration
    let Some(user) = UserStore::find_by_id(pool, user_id).await? else {
        return Err(eyre!("User session not resolved in database."));
    };

    let user_folder = user.media_folder.as_deref().unwrap_or("uploads");
    let user_media_dir = settings.media_root.join(user_folder);
    fs::create_dir_all(&user_media_dir)?;

    let source_file = settings.app_data_root.join("tus_uploads").join(&payload.upload_id);
    if !source_file.exists() {
        return Err(eyre!("TUS temporary upload file segment could not be found on disk."));
    }

    let dest_file = user_media_dir.join(&payload.filename);

    // Move the complete uploaded file to the user's permanent folder
    fs::rename(&source_file, &dest_file)?;

    // Clean up TUS-specific info files if they exist
    let info_file = source_file.with_extension("info");
    if info_file.exists() {
        fs::remove_file(info_file).ok();
    }

    // Determine the ingestion relative path
    let relative_path = std::path::Path::new(user_folder)
        .join(&payload.filename)
        .to_string_lossy()
        .to_string();

    info!(
        "TUS file transfer completed. File placed: {}. Queueing background workers...",
        relative_path
    );

    // 2. Trigger existing full-pipeline ingestion
    enqueue_full_ingest(pool, settings, &relative_path, user_id, None).await?;

    Ok(())
}