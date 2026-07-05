use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::remote_user::get_or_create_remote_user;
use app_state::MakeRelativePath;
use app_state::constants::ALBUM_IMPORT_FOLDER;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::{IngestMetadataPayload, enqueue_full_ingest};
use common_types::ImportAlbumItemPayload;
use serde_json::from_value;
use std::path::Path;
use std::slice;
use tokio::fs;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let media_root = &context.settings.ingest.media_root;

    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("ImportAlbumItem job is missing a payload"))?;
    let payload: ImportAlbumItemPayload = from_value(payload_value.clone())?;
    let user_id = job
        .user_id
        .ok_or_else(|| eyre!("ImportAlbumItem Job missing user_id"))?;
    let user_media_folder = UserStore::get_user_media_folder(&context.pool, user_id)
        .await?
        .ok_or_else(|| eyre!("User has no media folder configured"))?;
    let remote_host = payload
        .remote_url
        .host_str()
        .ok_or_else(|| eyre!("Remote URL is missing a host"))?;
    let remote_identity = format!("{}@{}", payload.remote_username, remote_host);
    let sanitized_identity = sanitize_filename::sanitize(&remote_identity);
    let relative_dir = Path::new(&user_media_folder)
        .join(ALBUM_IMPORT_FOLDER)
        .join(&sanitized_identity);
    let full_save_dir = media_root.join(&relative_dir);
    let filename = payload
        .remote_relative_path
        .split('/')
        .next_back()
        .ok_or_else(|| eyre!("Invalid relative path supplied."))?;
    let full_save_path = full_save_dir.join(filename);
    let relative_path = full_save_path.make_relative(&context.settings.ingest.media_root)?;
    fs::create_dir_all(&full_save_dir).await?;

    // Check if file already exists before downloading.
    if let Some(existing_id) =
        MediaItemStore::find_id_by_relative_path(&context.pool, &relative_path).await?
    {
        let mut tx = context.pool.begin().await?;
        AlbumStore::add_media_items(
            &mut *tx,
            &payload.local_album_id,
            slice::from_ref(&existing_id),
            user_id,
        )
        .await?;
        let remote_user_id = get_or_create_remote_user(&mut tx, user_id, &remote_identity).await?;
        MediaItemStore::update_remote_user_id(&mut *tx, &existing_id, remote_user_id).await?;
        tx.commit().await?;
        return Ok(JobResult::Done);
    }

    context
        .s2s_client
        .download_remote_file(
            &payload.token,
            &payload.remote_relative_path,
            &full_save_path,
        )
        .await?;

    enqueue_full_ingest(
        &context.pool,
        &context.settings.ingest,
        &relative_path,
        user_id,
        Some(IngestMetadataPayload {
            album_id: payload.local_album_id,
            remote_user_identity: remote_identity,
        }),
    )
    .await?;

    Ok(JobResult::Done)
}
