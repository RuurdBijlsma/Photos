use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::api::photos::removal::delete_item_and_thumbnails;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;

/// Handles the removal of a media item from the database and filesystem.
///
/// If the file still exists on disk, it is marked as deleted in the database (soft-delete).
/// If the file is missing, the database record and thumbnails are removed (hard-delete).
pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let Some(relative_path) = &job.relative_path else {
        return Err(eyre!("Remove job has no associated relative_path"));
    };
    let media_root = &context.settings.ingest.media_root;
    let file_path = media_root.join(relative_path);

    if file_path.exists() {
        // Soft delete: keep the original file and thumbnails.
        MediaItemStore::soft_delete_by_relative_path(&context.pool, relative_path).await?;
    } else {
        let thumbnail_root = &context.settings.ingest.thumbnails_root;
        // Hard delete: file is already gone, clean up DB and thumbnails.
        delete_item_and_thumbnails(&context.pool, thumbnail_root, relative_path).await?;
    }

    Ok(JobResult::Done)
}
