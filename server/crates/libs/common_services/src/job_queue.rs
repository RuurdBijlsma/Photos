use crate::database::jobs::{JobStatus, JobType};
use app_state::IngestSettings;
use bon::builder;
use chrono::{DateTime, Utc};
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use sqlx::{PgPool, PgTransaction};
use tracing::{info, warn};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IngestMetadataPayload {
    pub album_id: String,
    pub remote_user_identity: String,
}

/// Enqueues a job for a specific file, such as ingestion or removal.
///
/// # Errors
///
/// Returns an error if the database transaction fails.
#[builder]
pub async fn enqueue_job<T: Serialize + Send + Sync>(
    #[builder(start_fn)] pool: &PgPool,
    #[builder(start_fn)] settings: &IngestSettings,
    #[builder(start_fn)] job_type: JobType,
    #[builder(into)] relative_path: Option<String>,
    scheduled_at: Option<DateTime<Utc>>,
    user_id: Option<i32>,
    payload: Option<&T>,
) -> Result<bool> {
    let json_payload = payload.and_then(|p| to_value(p).ok());

    let mut tx = pool.begin().await?;

    if let Some(rel_path) = &relative_path {
        per_job_logic(&mut tx, job_type, rel_path).await?;
    }

    let is_video = relative_path
        .as_ref()
        .is_some_and(|p| settings.is_video_file(&settings.media_root.join(p)));
    let priority = job_type.get_priority(is_video);

    let result = sqlx::query!(
        r#"
        INSERT INTO jobs (relative_path, job_type, priority, user_id, payload, scheduled_at)
        VALUES ($1, $2, $3, $4, $5, COALESCE($6, now()))
        -- THIS PART MUST MATCH THE INDEX DEFINITION EXACTLY
        ON CONFLICT (job_type, coalesce(user_id, -1), coalesce(md5(payload::text), ''), coalesce(relative_path, ''))
        WHERE (status IN ('queued', 'running'))
        DO NOTHING
        "#,
        relative_path.as_deref(),
        job_type as JobType,
        priority,
        user_id,
        json_payload,
        scheduled_at,
    )
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    if result.rows_affected() == 0 {
        warn!(
            "Not enqueueing {:?} job {:?}, an active one already exists.",
            job_type, relative_path
        );
        return Ok(false);
    }

    info!(
        "Enqueued {:?} job {:?}, user_id: {:?}, payload: {:?}",
        job_type, relative_path, user_id, json_payload
    );

    Ok(true)
}

pub async fn enqueue_full_scan(
    pool: &PgPool,
    settings: &IngestSettings,
    target_user_id: i32,
) -> Result<()> {
    enqueue_job::<()>(pool, settings, JobType::Scan)
        .user_id(target_user_id)
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::DelayedScan)
        .user_id(target_user_id)
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::SyncThumbnails)
        .user_id(target_user_id)
        .call()
        .await?;
    Ok(())
}

/// Enqueues a full ingest and analysis job for a given file.
///
/// # Errors
///
/// Returns an error if any of the database operations fail.
pub async fn enqueue_full_ingest(
    pool: &PgPool,
    settings: &IngestSettings,
    relative_path: &str,
    user_id: i32,
    payload: Option<IngestMetadataPayload>,
) -> Result<()> {
    enqueue_job(pool, settings, JobType::IngestMetadata)
        .relative_path(relative_path)
        .user_id(user_id)
        .maybe_payload(payload.as_ref())
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::IngestThumbnails)
        .relative_path(relative_path)
        .user_id(user_id)
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::IngestAnalysis)
        .relative_path(relative_path)
        .user_id(user_id)
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::IngestLlm)
        .relative_path(relative_path)
        .user_id(user_id)
        .call()
        .await?;
    Ok(())
}

/// Applies some job logic specific to each job type.
///
/// * Enqueueing a remove job means existing ingest/analysis jobs for that file are be cancelled.
/// * Enqueueing an ingest/analysis job means existing remove jobs for that file are cancelled.
pub async fn per_job_logic(
    tx: &mut PgTransaction<'_>,
    job_type: JobType,
    relative_path: &str,
) -> Result<()> {
    match job_type {
        JobType::Remove => cancel_ingest_analysis_jobs(tx, relative_path).await?,
        JobType::IngestMetadata
        | JobType::IngestThumbnails
        | JobType::IngestAnalysis
        | JobType::IngestLlm => {
            cancel_remove_jobs(tx, relative_path).await?;
        }
        _ => (),
    }

    Ok(())
}

/// Cancel remove jobs for given `relative_path`.
async fn cancel_remove_jobs(tx: &mut PgTransaction<'_>, relative_path: &str) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled'
        WHERE
            relative_path = $1
            AND status IN ('queued', 'running')
            AND job_type IN ('remove')
        "#,
        relative_path
    )
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() > 0 {
        info!(
            "Cancelled {} queued/running remove job(s) for file: {}",
            result.rows_affected(),
            relative_path
        );
    }

    Ok(())
}

/// Cancels any queued ingest or analysis jobs.
///
/// # Errors
///
/// Returns an error if any of the database queries or the transaction commit fails.
async fn cancel_ingest_analysis_jobs(
    tx: &mut PgTransaction<'_>,
    relative_path: &str,
) -> Result<()> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled'
        WHERE
            relative_path = $1
            AND status IN ('queued', 'running')
            AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis', 'ingest_llm')
        "#,
        relative_path
    )
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() > 0 {
        info!(
            "Cancelled {} queued/running ingest_* job(s) for file: {}",
            result.rows_affected(),
            relative_path
        );
    }

    Ok(())
}

pub async fn bulk_enqueue_full_ingest(
    pool: &PgPool,
    ingest_settings: &IngestSettings,
    relative_paths: &[String],
    user_id: i32,
) -> Result<()> {
    if relative_paths.is_empty() {
        return Ok(());
    }

    let mut all_paths = Vec::with_capacity(relative_paths.len() * 3);
    let mut all_types = Vec::with_capacity(relative_paths.len() * 3);
    let mut all_priorities = Vec::with_capacity(relative_paths.len() * 3);

    for path in relative_paths {
        let file_path = ingest_settings.media_root.join(path);
        let is_video = ingest_settings.is_video_file(&file_path);

        for job_type in [
            JobType::IngestMetadata,
            JobType::IngestThumbnails,
            JobType::IngestAnalysis,
            JobType::IngestLlm,
        ] {
            all_paths.push(path.clone());
            all_types.push(job_type);
            all_priorities.push(job_type.get_priority(is_video));
        }
    }

    let status_str = format!("{:?}", JobStatus::Queued).to_lowercase();
    let statuses = vec![status_str; all_paths.len()];

    sqlx::query!(
        r#"
        INSERT INTO jobs (relative_path, job_type, priority, user_id, status)
        SELECT
            u.path,
            u.jt::job_type,
            u.pri,
            u.uid,
            u.stat::job_status
        FROM UNNEST($1::text[], $2::text[], $3::int[], $4::int[], $5::text[])
          AS u(path, jt, pri, uid, stat)
        ON CONFLICT (job_type, coalesce(user_id, -1), coalesce(md5(payload::text), ''), coalesce(relative_path, ''))
        WHERE (status IN ('queued', 'running'))
        DO NOTHING
        "#,
        &all_paths,
        &all_types as &[_],
        &all_priorities,
        &vec![user_id; all_paths.len()],
        &statuses,
    )
        .execute(pool)
        .await?;

    Ok(())
}
