use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use color_eyre::Result;
use common_services::database::jobs::{Job, JobScope, JobType};
use common_types::dev_constants::EMAIL;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use worker::handlers::JobResult;
use worker::jobs::management::{update_job_on_completion, update_job_on_failure};

#[derive(Deserialize)]
struct CancelResponse {
    cancelled: i64,
}

async fn insert_job(pool: &PgPool, user_id: i32, job_type: JobType, running: bool) -> Result<i64> {
    let id: i64 = sqlx::query_scalar!(
        r#"
        INSERT INTO jobs (relative_path, user_id, job_type, scope, phase, priority, status,
                          owner, started_at, last_heartbeat, scheduled_at)
        VALUES ($1, $2, $3, 'path'::job_scope, 20, 50,
                CASE WHEN $4 THEN 'running'::job_status ELSE 'queued'::job_status END,
                CASE WHEN $4 THEN 'integration-test' ELSE NULL END,
                CASE WHEN $4 THEN now() ELSE NULL END,
                CASE WHEN $4 THEN now() + interval '1 hour' ELSE now() END,
                now() + interval '1 hour')
        RETURNING id
        "#,
        format!("cancel-all/{user_id}/{job_type:?}.jpg"),
        user_id,
        job_type as JobType,
        running
    )
    .fetch_one(pool)
    .await?;
    Ok(id)
}

async fn status_of(pool: &PgPool, job_id: i64) -> Result<String> {
    let status = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM jobs WHERE id = $1"#,
        job_id
    )
    .fetch_one(pool)
    .await?;
    Ok(status)
}

pub async fn test_cancel_all_ingest_jobs(context: &TestContext) -> Result<()> {
    login(context).await?;
    let url = format!(
        "{}/api/jobs/ingest/cancel-all",
        context.settings.api.public_url
    );
    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user WHERE email = $1", EMAIL)
        .fetch_one(&context.pool)
        .await?;

    // Clear leftovers from earlier tests so the counts below are deterministic.
    sqlx::query!(
        "DELETE FROM jobs WHERE user_id = $1 \
         AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis') \
         AND status IN ('queued', 'running')",
        user_id
    )
    .execute(&context.pool)
    .await?;

    let other_user_id: i32 = sqlx::query_scalar!(
        "INSERT INTO app_user (email, password, name) \
         VALUES ('cancel-all-other@test.local', 'x', 'Other') RETURNING id"
    )
    .fetch_one(&context.pool)
    .await?;

    let metadata_id = insert_job(&context.pool, user_id, JobType::IngestMetadata, false).await?;
    let thumbnails_id = insert_job(&context.pool, user_id, JobType::IngestThumbnails, true).await?;
    let analysis_id = insert_job(&context.pool, user_id, JobType::IngestAnalysis, false).await?;
    let remove_id = insert_job(&context.pool, user_id, JobType::Remove, false).await?;
    let takeout_id = insert_job(&context.pool, user_id, JobType::IngestTakeoutBatch, false).await?;
    let other_user_job_id =
        insert_job(&context.pool, other_user_id, JobType::IngestMetadata, false).await?;

    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: CancelResponse = response.json().await?;
    assert_eq!(body.cancelled, 3);

    assert_eq!(status_of(&context.pool, metadata_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, thumbnails_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, analysis_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, remove_id).await?, "queued");
    assert_eq!(status_of(&context.pool, takeout_id).await?, "queued");
    assert_eq!(status_of(&context.pool, other_user_job_id).await?, "queued");

    // Idempotent: a second click cancels nothing.
    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: CancelResponse = response.json().await?;
    assert_eq!(body.cancelled, 0);

    let ids = [
        metadata_id,
        thumbnails_id,
        analysis_id,
        remove_id,
        takeout_id,
    ];
    sqlx::query!("DELETE FROM jobs WHERE id = ANY($1)", &ids[..])
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM app_user WHERE id = $1", other_user_id)
        .execute(&context.pool)
        .await?;

    Ok(())
}

pub async fn test_cancelled_job_is_not_resurrected(context: &TestContext) -> Result<()> {
    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user WHERE email = $1", EMAIL)
        .fetch_one(&context.pool)
        .await?;
    let job_id = insert_job(&context.pool, user_id, JobType::IngestMetadata, false).await?;

    sqlx::query!(
        "UPDATE jobs SET status = 'cancelled'::job_status, finished_at = now() WHERE id = $1",
        job_id
    )
    .execute(&context.pool)
    .await?;

    let job = Job {
        id: job_id,
        payload: None,
        relative_path: Some("cancel-all/guard.jpg".to_owned()),
        user_id: Some(user_id),
        job_type: JobType::IngestMetadata,
        scope: JobScope::Path,
        phase: 20,
        priority: 50,
        attempts: 0,
        max_attempts: 5,
        dependency_attempts: 0,
    };

    update_job_on_completion(&context.pool, &job, JobResult::DependencyReschedule).await?;
    update_job_on_completion(&context.pool, &job, JobResult::FileNotFound).await?;
    update_job_on_completion(
        &context.pool,
        &job,
        JobResult::StorageUnavailable("offline".to_owned()),
    )
    .await?;
    update_job_on_completion(&context.pool, &job, JobResult::Done).await?;
    update_job_on_failure(
        &context.pool,
        &job,
        &color_eyre::eyre::eyre!("simulated failure"),
    )
    .await?;

    let row = sqlx::query!(
        r#"SELECT status::text AS "status!", dependency_attempts, attempts, last_error, finished_at
           FROM jobs WHERE id = $1"#,
        job_id
    )
    .fetch_one(&context.pool)
    .await?;

    assert_eq!(row.status, "cancelled");
    assert_eq!(row.dependency_attempts, 0);
    assert_eq!(row.attempts, 0);
    assert!(row.last_error.is_none());
    assert!(row.finished_at.is_some());

    sqlx::query!("DELETE FROM jobs WHERE id = $1", job_id)
        .execute(&context.pool)
        .await?;

    Ok(())
}
