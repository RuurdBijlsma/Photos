use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::cache::ingest_cache::{get_ingest_cache, write_ingest_cache};
use crate::handlers::common::remote_user::get_or_create_remote_user;
use crate::handlers::common::utils::result_if_source_missing;
use crate::jobs::management::is_job_cancelled;
use color_eyre::eyre::Context;
use color_eyre::{Result, eyre::eyre};
use common_services::api::album::interfaces::AlbumSort;
use common_services::caching::hash_file;
use common_services::database::UpdateField;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::job_queue::IngestMetadataPayload;
use media_analyzer::MediaMetadata;
use serde_json::from_value;
use sqlx::PgPool;
use std::path::Path;
use tracing::debug;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let relative_path = job
        .relative_path
        .as_deref()
        .ok_or_else(|| eyre!("Ingest job has no associated relative_path"))?;
    let user_id = job
        .user_id
        .ok_or_else(|| eyre!("Ingest job has no associated user_id"))?;
    let media_root = &context.settings.ingest.media_root;
    let file_path = media_root.join(relative_path);
    if let Some(result) = result_if_source_missing(&context.pool, media_root, &file_path).await? {
        return Ok(result);
    }
    let file_hash = hash_file(&file_path)?;
    let media_info = get_media_info(context, &file_path, &file_hash).await?;
    let payload = if let Some(payload_json) = &job.payload {
        let payload: IngestMetadataPayload = from_value(payload_json.clone())?;
        Some(payload)
    } else {
        None
    };
    if is_job_cancelled(&context.pool, job.id).await? {
        return Ok(JobResult::Cancelled);
    }
    if let Some(result) = result_if_source_missing(&context.pool, media_root, &file_path).await? {
        return Ok(result);
    }
    store_media_item(&context.pool, user_id, relative_path, media_info, payload).await?;
    Ok(JobResult::Done)
}

/// Retrieves media analysis. Checks cache first, computes if missing.
async fn get_media_info(
    context: &WorkerContext,
    file_path: &Path,
    file_hash: &str,
) -> Result<MediaMetadata> {
    if context.settings.ingest.enable_cache
        && let Some(cached) =
            get_ingest_cache(&context.settings.ingest.cache_root, file_hash).await?
    {
        debug!("Using ingest cache for {:?}", file_path.file_name());
        return Ok(cached);
    }
    let media_info = context
        .media_analyzer
        .analyze_media(file_path)
        .await
        .wrap_err(file_path.to_string_lossy().to_string())?;
    if context.settings.ingest.enable_cache {
        write_ingest_cache(
            &context.settings.ingest.cache_root,
            file_hash,
            media_info.clone(),
        )
        .await?;
    }
    Ok(media_info)
}

async fn store_media_item(
    pool: &PgPool,
    user_id: i32,
    relative_path: &str,
    analyze_result: MediaMetadata,
    pending_payload: Option<IngestMetadataPayload>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let remote_user_id = if let Some(info) = &pending_payload {
        Some(get_or_create_remote_user(&mut tx, user_id, &info.remote_user_identity).await?)
    } else {
        None
    };
    let item_id = MediaItemStore::upsert(
        &mut tx,
        relative_path,
        user_id,
        remote_user_id,
        &analyze_result.into(),
    )
    .await?;
    if let Some(info) = &pending_payload {
        AlbumStore::add_media_items(
            &mut *tx,
            &info.album_id,
            std::slice::from_ref(&item_id),
            user_id,
        )
        .await?;
        if let Some(album) = AlbumStore::find_by_id(&mut *tx, &info.album_id).await?
            && album.sort_mode != AlbumSort::None
        {
            AlbumStore::sort_media_items(&mut tx, &info.album_id, album.sort_mode).await?;
        }
        if let Some(album) = AlbumStore::find_by_id(&mut *tx, &info.album_id).await?
            && album.thumbnail_id.is_none()
        {
            AlbumStore::update(
                &mut *tx,
                &info.album_id,
                None,
                UpdateField::Ignore,
                UpdateField::Value(item_id),
                None,
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
