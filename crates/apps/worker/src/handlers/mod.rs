use crate::context::WorkerContext;
use crate::jobs::heartbeat::start_heartbeat_loop;
use color_eyre::Result;
use common_services::database::jobs::{Job, JobType};

// Common
pub mod common;

// Jobs
pub mod calc_system_stats;
pub mod clean_db;
pub mod cluster_faces;
pub mod cluster_photos;
pub mod delayed_scan;
pub mod generate_daily_cards;
pub mod import_album_item;
pub mod ingest_analysis;
pub mod ingest_llm;
pub mod ingest_metadata;
pub mod ingest_thumbnails;
pub mod remove;
pub mod scan;
pub mod sync_thumbnails;
pub mod update_global_centroid;

/// The outcome of a job handler's execution.
#[derive(Debug, PartialEq, Eq)]
pub enum JobResult {
    Done,
    Cancelled,
    DependencyReschedule,
}

/// Dispatches a job to its corresponding handler and manages its lifecycle.
///
/// # Errors
///
/// This function will return an error if the specific job handler fails during execution.
pub async fn handle_job(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let heartbeat_handle = start_heartbeat_loop(&context.pool, job.id);

    let result = match job.job_type {
        JobType::IngestMetadata => ingest_metadata::handle(context, job).await,
        JobType::IngestThumbnails => ingest_thumbnails::handle(context, job).await,
        JobType::IngestAnalysis => ingest_analysis::handle(context, job).await,
        JobType::IngestLlm => ingest_llm::handle(context, job).await,
        JobType::Remove => remove::handle(context, job).await,
        JobType::Scan => scan::handle(context, job).await,
        JobType::CleanDb => clean_db::handle(context, job).await,
        JobType::DelayedScan => delayed_scan::handle(context, job).await,
        JobType::ClusterFaces => cluster_faces::handle(context, job).await,
        JobType::ClusterPhotos => cluster_photos::handle(context, job).await,
        JobType::SyncThumbnails => sync_thumbnails::handle(context, job).await,
        JobType::CalcSystemStats => calc_system_stats::handle(context, job).await,
        JobType::ImportAlbumItem => import_album_item::handle(context, job).await,
        JobType::GenerateDailyCards => generate_daily_cards::handle(context, job).await,
        JobType::UpdateGlobalCentroid => update_global_centroid::handle(context, job).await,
    };

    heartbeat_handle.abort();
    result
}
