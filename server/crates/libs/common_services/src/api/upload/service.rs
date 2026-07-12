use crate::api::upload::interfaces::UploadCompletePayload;
use crate::database::user_store::UserStore;
use crate::job_queue::enqueue_full_ingest;
use app_state::IngestSettings;
use color_eyre::eyre::{Result, eyre};
use sqlx::PgPool;
use std::fs;
use std::path::Path;
use tracing::info;

/// Safely moves a file from a temporary path to a permanent path,
/// handling cross-disk moves smoothly if rename fails.
fn safe_move_file(source: &Path, destination: &Path) -> Result<()> {
    // todo: make async
    if fs::rename(source, destination).is_err() {
        // todo: copy to temp direectory on the destination drive, then fs::rename to move it there instantly
        // otherwise scan can pick up a halfway copied file
        // idea to do this: copy to the destination folder, but with extension ".uploading" or something, so it isnt picked up because .uploading isnt a recognized extension
        fs::copy(source, destination)?;
        fs::remove_file(source)?;
    }
    Ok(())
}

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

    // todo: this unwrap_or is stupid
    let user_folder = user.media_folder.as_deref().unwrap_or("uploads");
    // todo: check for directory traversal
    let user_media_dir = settings.media_root.join(user_folder);
    fs::create_dir_all(&user_media_dir)?;

    let source_file = settings
        .app_data_root
        .join("tus_uploads")
        .join(&payload.upload_id);
    if !source_file.exists() {
        return Err(eyre!(
            "TUS temporary upload file segment could not be found on disk."
        ));
    }

    // todo: uploaded files should en up in {user_media_dir}/upload/{filename}
    let dest_file = user_media_dir.join(&payload.filename);

    // todo: if moving to dest file that already exists, it currently overwrites.
    // it should check if the file already exists, do a blake3 hash compare check if it's the same file, if it's the same file, don't copy it over
    // if it's a different file, make dest_file, something like {dest_file_base_filename}_1.{dest_file_ext}
    safe_move_file(&source_file, &dest_file)?;

    // Clean up TUS-specific metadata segment files if they exist
    let info_file = source_file.with_extension("info");
    if info_file.exists() {
        fs::remove_file(info_file).ok();
    }

    // Determine the ingestion relative path
    let relative_path = Path::new(user_folder)
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
