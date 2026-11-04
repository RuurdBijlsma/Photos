use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::takeout_import::{
    IngestWaitStage, TakeoutAlbumImport, has_pending_ingest_jobs,
};
use crate::handlers::update_global_centroid::refresh_global_centroid;
use color_eyre::Result;
use common_services::api::album::service::get_representative_thumbnail;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let import = TakeoutAlbumImport::from_job(job, "FinalizeTakeoutImport")?;

    if import.album_ids.is_empty() {
        return Ok(JobResult::Done);
    }

    if has_pending_ingest_jobs(
        &context.pool,
        import.user_id,
        &import.album_ids,
        IngestWaitStage::All,
    )
    .await?
    {
        return Ok(JobResult::DependencyReschedule);
    }

    // Analysis jobs are complete at this point, so refresh the centroid before selecting covers.
    refresh_global_centroid(&context.pool).await?;

    for album_id in import.album_ids {
        let mut tx = context.pool.begin().await?;
        if let Some(album) = AlbumStore::find_by_id(&mut *tx, &album_id).await? {
            let media_items =
                AlbumStore::list_sorted_media_items(&mut *tx, &album_id, album.sort_mode).await?;
            let media_item_ids: Vec<String> = media_items.into_iter().map(|item| item.id).collect();

            if let Some(thumbnail_id) =
                get_representative_thumbnail(&mut tx, &media_item_ids).await?
            {
                AlbumStore::update_thumbnail_if_not_user_selected(
                    &mut *tx,
                    &album_id,
                    &thumbnail_id,
                )
                .await?;
            }
        }
        tx.commit().await?;
    }

    Ok(JobResult::Done)
}
