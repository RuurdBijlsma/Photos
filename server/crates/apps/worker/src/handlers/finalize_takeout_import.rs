use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::update_global_centroid::refresh_global_centroid;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::api::album::service::get_representative_thumbnail;
use common_services::database::UpdateField;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::job_queue::TakeoutAlbumImportPayload;
use serde_json::from_value;
use sqlx::PgPool;

async fn has_pending_ingest_jobs(
    pool: &PgPool,
    user_id: i32,
    album_ids: &[String],
) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM jobs j
            WHERE j.user_id = $1
              AND j.status IN ('queued', 'running')
              AND (
                  j.job_type::text IN ('ingest_takeout_batch', 'prepare_takeout_import')
                  OR
                  (
                      j.job_type::text = 'ingest_metadata'
                      AND j.payload -> 'album_ids' ?| $2::text[]
                  )
                  OR (
                      j.job_type::text IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')
                      AND j.relative_path IN (
                          SELECT mi.relative_path
                          FROM album_media_item ami
                          JOIN media_item mi ON mi.id = ami.media_item_id
                          WHERE ami.album_id = ANY($2::text[])
                      )
                  )
              )
        )
        "#,
    )
    .bind(user_id)
    .bind(album_ids)
    .fetch_one(pool)
    .await?)
}

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let user_id = job
        .user_id
        .ok_or_else(|| eyre!("FinalizeTakeoutImport missing user_id"))?;
    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("FinalizeTakeoutImport missing payload"))?;
    let payload: TakeoutAlbumImportPayload = from_value(payload_value.clone())?;
    let mut album_ids = payload.album_ids;
    album_ids.sort_unstable();
    album_ids.dedup();

    if album_ids.is_empty() {
        return Ok(JobResult::Done);
    }

    if has_pending_ingest_jobs(&context.pool, user_id, &album_ids).await? {
        return Ok(JobResult::DependencyReschedule);
    }

    // Analysis jobs are complete at this point, so refresh the centroid before selecting covers.
    refresh_global_centroid(&context.pool).await?;

    for album_id in album_ids {
        let mut tx = context.pool.begin().await?;
        if let Some(album) = AlbumStore::find_by_id(&mut *tx, &album_id).await? {
            let media_items =
                AlbumStore::list_sorted_media_items(&mut *tx, &album_id, album.sort_mode).await?;
            let media_item_ids: Vec<String> = media_items.into_iter().map(|item| item.id).collect();

            if let Some(thumbnail_id) =
                get_representative_thumbnail(&mut tx, &media_item_ids).await?
            {
                AlbumStore::update(
                    &mut *tx,
                    &album_id,
                    None,
                    UpdateField::Ignore,
                    UpdateField::Value(thumbnail_id),
                    None,
                )
                .await?;
            }
        }
        tx.commit().await?;
    }

    Ok(JobResult::Done)
}
