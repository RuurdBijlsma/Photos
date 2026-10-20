use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::cache::ingest_cache::{get_ingest_cache, write_ingest_cache};
use crate::handlers::common::remote_user::get_or_create_remote_user;
use crate::handlers::common::takeout;
use crate::handlers::common::utils::result_if_source_missing;
use crate::jobs::management::is_job_cancelled;
use chrono::{DateTime, Utc};
use color_eyre::eyre::Context;
use color_eyre::{Result, eyre::eyre};
use common_services::caching::hash_file;
use common_services::database::UpdateField;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::database::media_item::media_item::CreateFullMediaItem;
use common_services::database::media_item_store::MediaItemStore;
use common_services::job_queue::IngestMetadataPayload;
use media_analyzer::MediaMetadata;
use serde_json::from_value;
use sqlx::PgPool;
use std::path::Path;
use tracing::{debug, warn};

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
    if tokio::fs::metadata(&file_path)
        .await
        .map_or(true, |m| m.len() == 0)
    {
        warn!("File is 0 bytes: {relative_path}. Cancelling ingest job.");
        return Ok(JobResult::Cancelled);
    }
    let file_hash = hash_file(&file_path)?;
    let mut media_info = get_media_info(context, &file_path, &file_hash).await?;
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
    let fallback_taken_at = payload
        .as_ref()
        .and_then(|info| info.fallback_taken_at);
    let override_taken_at =
        takeout::resolve_override(&mut media_info, &file_path, fallback_taken_at).await?;
    store_media_item(
        &context.pool,
        user_id,
        relative_path,
        media_info,
        override_taken_at,
        payload,
    )
    .await?;
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
    override_taken_at: Option<DateTime<Utc>>,
    pending_payload: Option<IngestMetadataPayload>,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    let remote_user_id = if let Some(info) = &pending_payload
        && let Some(remote_identity) = &info.remote_user_identity
    {
        Some(get_or_create_remote_user(&mut tx, user_id, remote_identity).await?)
    } else {
        None
    };
    let mut media_item: CreateFullMediaItem = analyze_result.into();
    if let Some(override_taken_at) = override_taken_at {
        takeout::apply_to_media_item(&mut media_item, override_taken_at);
    }

    let item_id =
        MediaItemStore::upsert(&mut tx, relative_path, user_id, remote_user_id, &media_item)
            .await?;
    if let Some(info) = &pending_payload {
        // Stable lock order keeps concurrent multi-album ingests from deadlocking.
        let mut album_ids = info.album_ids.clone();
        album_ids.sort_unstable();
        for album_id in &album_ids {
            AlbumStore::add_media_items(&mut tx, album_id, std::slice::from_ref(&item_id), user_id)
                .await?;
            if let Some(album) = AlbumStore::find_by_id(&mut *tx, album_id).await?
                && album.thumbnail_id.is_none()
            {
                AlbumStore::update(
                    &mut *tx,
                    album_id,
                    None,
                    UpdateField::Ignore,
                    UpdateField::Value(item_id.clone()),
                    None,
                )
                .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}
