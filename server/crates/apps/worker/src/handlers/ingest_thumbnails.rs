use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::cache::thumbnail_cache::{get_thumbnail_cache, write_thumbnail_cache};
use crate::handlers::common::utils::result_if_source_missing;
use crate::jobs::management::is_job_cancelled;
use color_eyre::{Result, eyre::eyre};
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use generate_thumbnails::{ThumbnailConfig, generate_thumbnails};
use serde_json::Value;
use sqlx::PgPool;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tracing::{debug, warn};

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let relative_path = job
        .relative_path
        .as_deref()
        .ok_or_else(|| eyre!("Ingest job has no associated relative_path"))?;
    let media_root = &context.settings.ingest.media_root;
    let file_path = media_root.join(relative_path);
    let Some(row) = sqlx::query!(
        "SELECT hash, media_item_id, use_panorama_viewer, orientation, mf.is_motion_photo
        FROM media_item
                 INNER JOIN media_features mf ON mf.media_item_id = media_item.id
        WHERE relative_path = $1",
        relative_path
    )
    .fetch_optional(&context.pool)
    .await?
    else {
        warn!("IngestThumbnail was called but no media_item row exists for {relative_path}");
        return Ok(JobResult::Cancelled);
    };
    if let Some(result) = result_if_source_missing(&context.pool, media_root, &file_path).await? {
        return Ok(result);
    }
    if tokio::fs::metadata(&file_path).await.map_or(true, |m| m.len() == 0) {
        warn!("File is 0 bytes: {relative_path}. Cancelling thumbnail generation.");
        return Ok(JobResult::Cancelled);
    }
    process_thumbnails(
        context,
        &file_path,
        &row.hash,
        &row.media_item_id,
        ThumbnailConfig {
            extract_motion_photo: row.is_motion_photo,
            orientation: row.orientation,
            generate_panorama_tiles: row.use_panorama_viewer,
        },
    )
    .await?;
    if is_job_cancelled(&context.pool, job.id).await? {
        return Ok(JobResult::Cancelled);
    }
    if let Some(result) = result_if_source_missing(&context.pool, media_root, &file_path).await? {
        return Ok(result);
    }
    sqlx::query!(
        r"
        UPDATE media_item
        SET has_thumbnails = TRUE,
        updated_at = now()
        WHERE id = $1;",
        &row.media_item_id
    )
    .execute(&context.pool)
    .await?;
    Ok(JobResult::Done)
}

async fn store_panorama_config(
    pool: &PgPool,
    pano_sub_folder: &Path,
    media_item_id: &str,
) -> Result<()> {
    let pano_config_path = pano_sub_folder.join("config.json");
    if !pano_config_path.exists() {
        return Err(eyre!("Pano config file does not exist"));
    }
    let file = File::open(&pano_config_path)?;
    let reader = BufReader::new(file);
    let json: Value = serde_json::from_reader(reader)?;
    MediaItemStore::upsert_panorama_config(pool, media_item_id, &json).await?;
    Ok(())
}

/// Handles thumbnail creation. Checks cache first, generates if missing.
async fn process_thumbnails(
    context: &WorkerContext,
    file_path: &Path,
    file_hash: &str,
    media_item_id: &str,
    thumbnail_config: ThumbnailConfig,
) -> Result<()> {
    let thumbnails_out_folder = context.settings.ingest.thumbnails_root.join(media_item_id);
    let pano_out_folder = context.settings.ingest.pano_root.join(media_item_id);

    // Cache Check
    if context.settings.ingest.enable_cache
        && get_thumbnail_cache(
            &context.settings.ingest.cache_root,
            file_hash,
            &thumbnails_out_folder,
            &pano_out_folder,
            thumbnail_config.generate_panorama_tiles,
        )
        .await?
    {
        debug!("Using thumbnail cache for {:?}", file_path.file_name());
    } else {
        // Cache Miss: Generate
        let result = generate_thumbnails(
            &context.settings.ingest,
            file_path,
            &thumbnails_out_folder,
            &pano_out_folder,
            thumbnail_config.clone(),
        )
        .await?;

        if !result.pano_success {
            warn!("PANO FAILURE: {}", media_item_id);
            sqlx::query!(
                r"
                UPDATE media_item
                SET use_panorama_viewer = FALSE,
                updated_at = now()
                WHERE id = $1;",
                &media_item_id
            )
            .execute(&context.pool)
            .await?;
        }

        if !result.motion_success {
            warn!("MOTION FAILURE: {}", media_item_id);
            sqlx::query!(
                r"
                UPDATE media_features
                SET is_motion_photo = FALSE
                WHERE media_item_id = $1;",
                &media_item_id
            )
            .execute(&context.pool)
            .await?;
        }

        // Write Cache
        if context.settings.ingest.enable_cache {
            write_thumbnail_cache(
                &context.settings.ingest.cache_root,
                file_hash,
                &thumbnails_out_folder,
                &pano_out_folder,
                thumbnail_config.generate_panorama_tiles,
            )
            .await?;
        }
    }

    if thumbnail_config.generate_panorama_tiles
        && let Err(e) = store_panorama_config(&context.pool, &pano_out_folder, media_item_id).await
    {
        warn!(
            "Couldn't store panorama config for {}, {e}",
            file_path.display()
        );
    }

    Ok(())
}
