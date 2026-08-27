use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::cache::analysis_cache::{get_analysis_cache, write_analysis_cache};
use crate::handlers::common::utils::{get_images_to_analyze, result_if_source_missing};
use crate::jobs::management::is_job_cancelled;
use color_eyre::eyre::{Result, eyre};
use common_services::database::jobs::Job;
use common_services::database::media_item::media_item::MediaItemIdAndHash;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::visual_analysis_store::VisualAnalysisStore;
use common_types::ml_analysis::MLFastAnalysis;
use std::path::Path;
use tracing::{debug, info, warn};

/// Handles the analysis of a given job.
///
/// # Errors
///
/// This function will return an error if the media analysis fails,
/// or if there are issues with database operations.
pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let Some(relative_path) = &job.relative_path else {
        return Err(eyre!("Ingest job has no associated relative_path"));
    };
    let media_root = &context.settings.ingest.media_root;
    let file_path = media_root.join(relative_path);
    if let Some(result) = result_if_source_missing(&context.pool, media_root, &file_path).await? {
        return Ok(result);
    }
    if !context
        .settings
        .ingest
        .file_is_ingested(&context.pool, &file_path)
        .await?
    {
        info!(
            "File {} is not ingested yet, rescheduling analysis.",
            &relative_path
        );
        return Ok(JobResult::DependencyReschedule);
    }
    let MediaItemIdAndHash {
        media_item_id,
        hash,
    } = MediaItemStore::find_id_and_hash_by_relative_path(&context.pool, relative_path)
        .await?
        .ok_or_else(|| eyre!("Could not find media item by relative_path."))?;
    let analyses = get_analysis_data(context, &file_path, &media_item_id, &hash).await?;
    let user_id = if let Some(uid) = job.user_id {
        uid
    } else {
        MediaItemStore::find_user_by_id(&context.pool, &media_item_id)
            .await?
            .ok_or_else(|| eyre!("Invalid media item linked to visual analysis"))?
    };
    save_results(
        context,
        job.id,
        &media_item_id,
        user_id,
        &analyses,
        &file_path,
    )
    .await
}

async fn get_analysis_data(
    context: &WorkerContext,
    file_path: &Path,
    media_item_id: &str,
    file_hash: &str,
) -> Result<Vec<MLFastAnalysis>> {
    if context.settings.ingest.enable_cache
        && let Some(cached_analysis) =
            get_analysis_cache(&context.settings.ingest.cache_root, file_hash).await?
    {
        debug!("Using analysis cache for {}", media_item_id);
        return Ok(cached_analysis);
    }
    let analyses = compute_analysis(context, file_path, media_item_id).await?;
    if context.settings.ingest.enable_cache {
        write_analysis_cache(&context.settings.ingest.cache_root, file_hash, &analyses).await?;
    }

    Ok(analyses)
}

/// Performs the actual ML analysis by spawning blocking tasks.
async fn compute_analysis(
    context: &WorkerContext,
    file_path: &Path,
    media_item_id: &str,
) -> Result<Vec<MLFastAnalysis>> {
    let images_to_analyze = get_images_to_analyze(
        context,
        file_path,
        media_item_id,
        &context.settings.ingest.thumbnails.video_options.percentages,
        None,
    );
    let mut analyses = Vec::new();

    for (percentage, image_path) in images_to_analyze {
        let analyzer_settings = context.settings.ingest.analyzer.clone();
        // This spawn blocking -> block_on is needed because analyze image does work on the
        // main thread and this disturbs the integration test.
        let analyzer = context.visual_analyzer.clone().ok_or_else(|| {
            eyre!("No visual_analyzer on worker that picked up ingest_analysis job")
        })?;
        let analysis_result = tokio::task::spawn_blocking(move || {
            analyzer.fast_image_analysis(&analyzer_settings, &image_path, percentage)
        })
        .await??;

        analyses.push(analysis_result);
    }

    Ok(analyses)
}

/// Saves the analysis results to the database in a transaction.
/// Also checks for cancellation one last time before committing.
async fn save_results(
    context: &WorkerContext,
    job_id: i64,
    media_item_id: &str,
    user_id: i32,
    analyses: &[MLFastAnalysis],
    file_path: &Path,
) -> Result<JobResult> {
    let mut tx = context.pool.begin().await?;

    // Check cancellation or file existence one last time inside the transaction
    if is_job_cancelled(&mut *tx, job_id).await? {
        return Ok(JobResult::Cancelled);
    }
    if let Some(result) = result_if_source_missing(
        &context.pool,
        &context.settings.ingest.media_root,
        file_path,
    )
    .await?
    {
        return Ok(result);
    }
    for analysis in analyses {
        VisualAnalysisStore::create_fast_analysis(
            &mut tx,
            media_item_id,
            user_id,
            &analysis.to_owned().into(),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(JobResult::Done)
}
