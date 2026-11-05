use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::jobs::Job;
use common_services::job_queue::TakeoutAlbumImportPayload;
use serde_json::from_value;
use sqlx::PgPool;

pub struct TakeoutAlbumImport {
    pub user_id: i32,
    pub album_ids: Vec<String>,
}

impl TakeoutAlbumImport {
    pub fn from_job(job: &Job, job_name: &str) -> Result<Self> {
        let user_id = job
            .user_id
            .ok_or_else(|| eyre!("{job_name} missing user_id"))?;
        let payload_value = job
            .payload
            .as_ref()
            .ok_or_else(|| eyre!("{job_name} missing payload"))?;
        let payload: TakeoutAlbumImportPayload = from_value(payload_value.clone())?;
        let mut album_ids = payload.album_ids;
        album_ids.sort_unstable();
        album_ids.dedup();

        Ok(Self { user_id, album_ids })
    }
}

#[derive(Clone, Copy)]
pub enum IngestWaitStage {
    Metadata,
    All,
}

pub async fn has_pending_ingest_jobs(
    pool: &PgPool,
    user_id: i32,
    album_ids: &[String],
    wait_stage: IngestWaitStage,
) -> Result<bool> {
    let wait_for_all_stages = matches!(wait_stage, IngestWaitStage::All);
    Ok(sqlx::query_scalar!(
        r#"
        WITH album_paths AS (
            SELECT mi.relative_path
            FROM album_media_item ami
            JOIN media_item mi ON mi.id = ami.media_item_id
            WHERE ami.album_id = ANY($2::text[])
        )
        SELECT EXISTS (
            SELECT 1
            FROM jobs j
            WHERE j.user_id = $1
              AND j.status IN ('queued', 'running')
              AND (
                  j.job_type::text = 'ingest_takeout_batch'
                  OR (
                      $3::boolean
                      AND j.job_type::text = 'prepare_takeout_import'
                  )
                  OR (
                      j.job_type::text = 'ingest_metadata'
                      AND (
                          j.payload -> 'album_ids' ?| $2::text[]
                          OR j.relative_path IN (SELECT relative_path FROM album_paths)
                      )
                  )
                  OR (
                      $3::boolean
                      AND j.job_type::text IN ('ingest_thumbnails', 'ingest_analysis')
                      AND j.relative_path IN (SELECT relative_path FROM album_paths)
                  )
              )
        ) AS "has_pending!"
        "#,
        user_id,
        album_ids,
        wait_for_all_stages
    )
    .fetch_one(pool)
    .await?)
}
