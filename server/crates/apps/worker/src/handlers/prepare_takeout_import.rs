use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::UpdateField;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::job_queue::TakeoutAlbumImportPayload;
use serde_json::from_value;
use sqlx::PgPool;

async fn has_pending_metadata(pool: &PgPool, user_id: i32, album_ids: &[String]) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM jobs j
            WHERE j.user_id = $1
              AND j.status IN ('queued', 'running')
              AND (
                  j.job_type::text = 'ingest_takeout_batch'
                  OR (
                      j.job_type::text = 'ingest_metadata'
                      AND j.payload -> 'album_ids' ?| $2::text[]
                  )
                  OR (
                      j.job_type::text = 'ingest_metadata'
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
        .ok_or_else(|| eyre!("PrepareTakeoutImport missing user_id"))?;
    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("PrepareTakeoutImport missing payload"))?;
    let payload: TakeoutAlbumImportPayload = from_value(payload_value.clone())?;
    let mut album_ids = payload.album_ids;
    album_ids.sort_unstable();
    album_ids.dedup();

    if album_ids.is_empty() {
        return Ok(JobResult::Done);
    }

    if has_pending_metadata(&context.pool, user_id, &album_ids).await? {
        return Ok(JobResult::DependencyReschedule);
    }

    for album_id in album_ids {
        let mut tx = context.pool.begin().await?;
        if let Some(thumbnail_id) =
            AlbumStore::find_middle_media_item_id(&mut *tx, &album_id).await?
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
        tx.commit().await?;
    }

    Ok(JobResult::Done)
}
