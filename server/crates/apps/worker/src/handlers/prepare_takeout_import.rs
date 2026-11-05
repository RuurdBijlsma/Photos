use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::takeout_import::{
    IngestWaitStage, TakeoutAlbumImport, has_pending_ingest_jobs,
};
use color_eyre::Result;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let import = TakeoutAlbumImport::from_job(job, "PrepareTakeoutImport")?;

    if import.album_ids.is_empty() {
        return Ok(JobResult::Done);
    }

    if has_pending_ingest_jobs(
        &context.pool,
        import.user_id,
        &import.album_ids,
        IngestWaitStage::Metadata,
    )
    .await?
    {
        return Ok(JobResult::DependencyReschedule);
    }

    for album_id in import.album_ids {
        if let Some(thumbnail_id) =
            AlbumStore::find_middle_media_item_id(&context.pool, &album_id).await?
        {
            AlbumStore::update_thumbnail_if_not_user_selected(
                &context.pool,
                &album_id,
                &thumbnail_id,
            )
            .await?;
        }
    }

    Ok(JobResult::Done)
}
