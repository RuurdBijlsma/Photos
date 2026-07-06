use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::cache::llm_cache::{get_llm_cache, write_llm_cache};
use crate::handlers::common::utils::get_images_to_analyze;
use crate::jobs::management::is_job_cancelled;
use color_eyre::eyre::{Result, eyre};
use common_services::caching::hash_file;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::visual_analysis_store::VisualAnalysisStore;
use common_types::ml_analysis::MLChatAnalysis;
use std::path::Path;
use tracing::{debug, info};

/// Handles the llm analysis of a given job.
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
    if !file_path.exists() {
        return Ok(JobResult::Cancelled);
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
    let media_item_id = MediaItemStore::find_id_by_relative_path(&context.pool, relative_path)
        .await?
        .ok_or_else(|| eyre!("Could not find media item by relative_path."))?;
    let (percentages, v_ids): (Vec<_>, Vec<_>) = sqlx::query!(
        "SELECT percentage, id
        FROM visual_analysis
        WHERE media_item_id = $1",
        media_item_id
    )
    .fetch_all(&context.pool)
    .await?
    .iter()
    .map(|row| (row.percentage, row.id))
    .unzip();
    let percentages = percentages.iter().map(|p| *p as u64).collect::<Vec<_>>();

    let llm_analyses =
        get_cached_llm_data(context, &file_path, &media_item_id, &percentages).await?;

    let result: Vec<(i64, MLChatAnalysis)> = v_ids.into_iter().zip(llm_analyses).collect();

    save_llm_results(context, job.id, result, &file_path).await
}

async fn get_cached_llm_data(
    context: &WorkerContext,
    file_path: &Path,
    media_item_id: &str,
    percentages: &[u64],
) -> Result<Vec<MLChatAnalysis>> {
    let file_hash = hash_file(file_path)?;
    if context.settings.ingest.enable_cache
        && let Some(cached_analysis) =
            get_llm_cache(&context.settings.ingest.cache_root, &file_hash).await?
    {
        debug!(
            "Using analysis cache for {}",
            file_path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default()
        );
        return Ok(cached_analysis);
    }
    let analyses = get_llm_data(context, file_path, media_item_id, percentages).await?;
    if context.settings.ingest.enable_cache {
        write_llm_cache(&context.settings.ingest.cache_root, &file_hash, &analyses).await?;
    }

    Ok(analyses)
}

/// Performs the actual ML analysis by spawning blocking tasks.
async fn get_llm_data(
    context: &WorkerContext,
    file_path: &Path,
    media_item_id: &str,
    percentages: &[u64],
) -> Result<Vec<MLChatAnalysis>> {
    let images_to_analyze =
        get_images_to_analyze(context, file_path, media_item_id, percentages, None);
    let mut analyses = Vec::new();

    for (percentage, image_path) in images_to_analyze {
        let analysis_result = context
            .visual_analyzer
            .llm_analysis(&context.settings.ingest.analyzer, &image_path, percentage)
            .await?;
        analyses.push(analysis_result);
    }

    Ok(analyses)
}

/// Saves the analysis results to the database in a transaction.
/// Also checks for cancellation one last time before committing.
async fn save_llm_results(
    context: &WorkerContext,
    job_id: i64,
    results: Vec<(i64, MLChatAnalysis)>,
    file_path: &Path,
) -> Result<JobResult> {
    let mut tx = context.pool.begin().await?;

    // Check cancellation or file existence one last time inside the transaction
    if is_job_cancelled(&mut *tx, job_id).await? || !file_path.exists() {
        return Ok(JobResult::Cancelled);
    }
    for (visual_analysis_id, ml_chat_analysis) in results {
        VisualAnalysisStore::add_llm_analysis(
            &mut tx,
            visual_analysis_id,
            &ml_chat_analysis.llm_classification.clone().into(),
            ml_chat_analysis.quality_judge.clone().map(Into::into),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(JobResult::Done)
}
