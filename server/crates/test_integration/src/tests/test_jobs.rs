use crate::runner::context::test_context::TestContext;
use crate::test_helpers::login;
use color_eyre::Result;
use common_services::database::jobs::{Job, JobScope, JobType};
use common_services::job_queue::{bulk_enqueue_full_ingest, enqueue_job};
use common_types::dev_constants::EMAIL;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Barrier;
use worker::handlers::JobResult;
use worker::handlers::sync_ingest::repair_incomplete_ingest;
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

async fn insert_global_scan(pool: &PgPool) -> Result<i64> {
    let id: i64 = sqlx::query_scalar!(
        r#"
        INSERT INTO jobs (job_type, scope, phase, priority, status, scheduled_at)
        VALUES ('scan'::job_type, 'global'::job_scope, 20, 50, 'queued'::job_status,
                now() + interval '1 hour')
        RETURNING id
        "#
    )
    .fetch_one(pool)
    .await?;
    Ok(id)
}

async fn insert_incomplete_media_item(
    pool: &PgPool,
    template_id: &str,
    id: &str,
    relative_path: &str,
    has_thumbnails: bool,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO media_item (
            id, user_id, hash, relative_path, filename, width, height, is_video,
            duration_ms, orientation, taken_at_local, og_taken_at_local, taken_at_utc,
            sort_timestamp, timezone_name, timezone_offset_seconds, og_timezone_offset_seconds,
            use_panorama_viewer, has_thumbnails, user_caption, deleted, missing_since
        )
        SELECT $1, user_id, hash, $2, $3, width, height, is_video,
               duration_ms, orientation, taken_at_local, og_taken_at_local, taken_at_utc,
               sort_timestamp, timezone_name, timezone_offset_seconds, og_timezone_offset_seconds,
               use_panorama_viewer, $4, user_caption, false, NULL
        FROM media_item WHERE id = $5
        "#,
        id,
        relative_path,
        relative_path,
        has_thumbnails,
        template_id
    )
    .execute(pool)
    .await?;
    Ok(())
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
        "DELETE FROM jobs \
         WHERE status IN ('queued', 'running') \
           AND ( \
               (user_id = $1 AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')) \
               OR job_type IN ('scan', 'sync_ingest') \
           )",
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
    let scan_id = insert_job(&context.pool, user_id, JobType::Scan, false).await?;
    let sync_id = insert_job(&context.pool, user_id, JobType::SyncIngest, true).await?;
    let global_scan_id = insert_global_scan(&context.pool).await?;
    let remove_id = insert_job(&context.pool, user_id, JobType::Remove, false).await?;
    let takeout_id = insert_job(&context.pool, user_id, JobType::IngestTakeoutBatch, false).await?;
    let other_user_job_id =
        insert_job(&context.pool, other_user_id, JobType::IngestMetadata, false).await?;

    let max_id: i64 = sqlx::query_scalar!(r#"SELECT COALESCE(MAX(id), 0) AS "max_id!" FROM jobs"#)
        .fetch_one(&context.pool)
        .await?;
    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let body: CancelResponse = response.json().await?;
    assert_eq!(body.cancelled, 5);

    assert_eq!(status_of(&context.pool, metadata_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, thumbnails_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, analysis_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, scan_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, sync_id).await?, "running");
    assert_eq!(status_of(&context.pool, global_scan_id).await?, "cancelled");
    assert_eq!(status_of(&context.pool, remove_id).await?, "queued");
    assert_eq!(status_of(&context.pool, takeout_id).await?, "queued");
    assert_eq!(status_of(&context.pool, other_user_job_id).await?, "queued");

    // The active repair is left alone, and a fresh global repair is started.
    let repair_count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*)::bigint AS "repair_count!" FROM jobs
           WHERE id > $1 AND job_type = 'sync_ingest' AND relative_path IS NULL"#,
        max_id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(repair_count, 1);

    let ids = [
        metadata_id,
        thumbnails_id,
        analysis_id,
        scan_id,
        sync_id,
        global_scan_id,
        remove_id,
        takeout_id,
    ];
    sqlx::query!("DELETE FROM jobs WHERE id = ANY($1)", &ids[..])
        .execute(&context.pool)
        .await?;
    sqlx::query!(
        "DELETE FROM jobs WHERE id > $1 AND job_type = 'sync_ingest'",
        max_id
    )
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

pub async fn test_bulk_enqueue_full_ingest_concurrent_overlap(context: &TestContext) -> Result<()> {
    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user WHERE email = $1", EMAIL)
        .fetch_one(&context.pool)
        .await?;
    let prefix = "deadlock-test/";
    let paths: Vec<String> = (0..1000).map(|i| format!("{prefix}{i}.jpg")).collect();
    let forwards: Vec<_> = paths.iter().map(|path| (path.clone(), None)).collect();
    let backwards: Vec<_> = forwards.iter().rev().cloned().collect();
    let barrier = Arc::new(Barrier::new(2));

    let forwards_barrier = Arc::clone(&barrier);
    let forwards_task = async {
        forwards_barrier.wait().await;
        bulk_enqueue_full_ingest(&context.pool, &context.settings.ingest, &forwards, user_id).await
    };
    let backwards_barrier = Arc::clone(&barrier);
    let backwards_task = async {
        backwards_barrier.wait().await;
        bulk_enqueue_full_ingest(&context.pool, &context.settings.ingest, &backwards, user_id).await
    };

    let (forwards_result, backwards_result) = tokio::join!(forwards_task, backwards_task);

    sqlx::query!(
        "DELETE FROM jobs WHERE user_id = $1 AND relative_path LIKE $2",
        user_id,
        format!("{prefix}%")
    )
    .execute(&context.pool)
    .await?;

    forwards_result?;
    backwards_result?;
    Ok(())
}

pub async fn test_sync_ingest_is_global_singleton(context: &TestContext) -> Result<()> {
    sqlx::query!(
        "DELETE FROM jobs WHERE job_type = 'sync_ingest' \
         AND status IN ('queued', 'running')"
    )
    .execute(&context.pool)
    .await?;

    let other_user_id: i32 = sqlx::query_scalar!(
        "INSERT INTO app_user (email, password, name) \
         VALUES ('sync-singleton-other@test.local', 'x', 'Other') RETURNING id"
    )
    .fetch_one(&context.pool)
    .await?;
    let second_user_id: i32 = sqlx::query_scalar!(
        "INSERT INTO app_user (email, password, name) \
         VALUES ('sync-singleton-second@test.local', 'x', 'Second') RETURNING id"
    )
    .fetch_one(&context.pool)
    .await?;

    let first = enqueue_job::<()>(&context.pool, &context.settings.ingest, JobType::SyncIngest)
        .user_id(other_user_id)
        .call()
        .await?;
    let second = enqueue_job::<()>(&context.pool, &context.settings.ingest, JobType::SyncIngest)
        .user_id(second_user_id)
        .call()
        .await?;
    assert!(first);
    assert!(!second);

    let rows = sqlx::query!(
        "SELECT id, user_id FROM jobs \
         WHERE job_type = 'sync_ingest' AND status IN ('queued', 'running')"
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].user_id.is_none());

    sqlx::query!("DELETE FROM jobs WHERE id = $1", rows[0].id)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM app_user WHERE id = $1", other_user_id)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM app_user WHERE id = $1", second_user_id)
        .execute(&context.pool)
        .await?;

    Ok(())
}

pub async fn test_scan_enqueues_sync_ingest(context: &TestContext) -> Result<()> {
    login(context).await?;
    let url = format!("{}/api/jobs/ingest/scan", context.settings.api.public_url);

    // Clear active scan/sync rows so dedupe cannot hide the new jobs.
    sqlx::query!(
        "DELETE FROM jobs WHERE job_type IN ('scan', 'sync_ingest') \
         AND status IN ('queued', 'running')"
    )
    .execute(&context.pool)
    .await?;

    let max_id: i64 = sqlx::query_scalar!(r#"SELECT COALESCE(MAX(id), 0) AS "max_id!" FROM jobs"#)
        .fetch_one(&context.pool)
        .await?;

    let response = context.http_client.post(&url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);

    let rows = sqlx::query!(
        r#"
        SELECT job_type::text AS "job_type!", COUNT(*)::bigint AS "count!"
        FROM jobs
        WHERE id > $1 AND job_type IN ('scan', 'sync_ingest')
        GROUP BY job_type
        "#,
        max_id
    )
    .fetch_all(&context.pool)
    .await?;

    assert!(
        rows.iter()
            .any(|row| row.job_type == "scan" && row.count == 1)
    );
    assert!(
        rows.iter()
            .any(|row| row.job_type == "sync_ingest" && row.count == 1)
    );

    sqlx::query!(
        "DELETE FROM jobs WHERE id > $1 AND job_type IN ('scan', 'sync_ingest')",
        max_id
    )
    .execute(&context.pool)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn test_sync_ingest_repairs_incomplete_items(context: &TestContext) -> Result<()> {
    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user WHERE email = $1", EMAIL)
        .fetch_one(&context.pool)
        .await?;
    let template = sqlx::query!(
        "SELECT id, relative_path FROM media_item WHERE user_id = $1 AND is_video = false LIMIT 1",
        user_id
    )
    .fetch_one(&context.pool)
    .await?;

    let thumbs_path = "sync-ingest-test-thumbs.jpg";
    let analysis_path = "sync-ingest-test-analysis.jpg";
    let missing_path = "sync-ingest-test-missing.jpg";
    let blocked_thumbs_path = "sync-ingest-test-blocked-thumbs.jpg";
    let blocked_analysis_path = "sync-ingest-test-blocked-analysis.jpg";
    let source = context
        .settings
        .ingest
        .media_root
        .join(&template.relative_path);

    insert_incomplete_media_item(
        &context.pool,
        &template.id,
        "synctest01",
        thumbs_path,
        false,
    )
    .await?;
    insert_incomplete_media_item(
        &context.pool,
        &template.id,
        "synctest02",
        analysis_path,
        true,
    )
    .await?;
    insert_incomplete_media_item(
        &context.pool,
        &template.id,
        "synctest03",
        missing_path,
        false,
    )
    .await?;
    insert_incomplete_media_item(
        &context.pool,
        &template.id,
        "synctest04",
        blocked_thumbs_path,
        false,
    )
    .await?;
    insert_incomplete_media_item(
        &context.pool,
        &template.id,
        "synctest05",
        blocked_analysis_path,
        true,
    )
    .await?;
    sqlx::query!("UPDATE media_item SET thumbnail_repair_blocked = true WHERE id = 'synctest04'")
        .execute(&context.pool)
        .await?;
    sqlx::query!("UPDATE media_item SET analysis_repair_blocked = true WHERE id = 'synctest05'")
        .execute(&context.pool)
        .await?;

    // Insert the rows first so the file watcher sees them and does not enqueue ingest jobs.
    fs::copy(
        &source,
        context.settings.ingest.media_root.join(thumbs_path),
    )
    .await?;
    fs::copy(
        &source,
        context.settings.ingest.media_root.join(analysis_path),
    )
    .await?;
    fs::copy(
        &source,
        context.settings.ingest.media_root.join(blocked_thumbs_path),
    )
    .await?;
    fs::copy(
        &source,
        context
            .settings
            .ingest
            .media_root
            .join(blocked_analysis_path),
    )
    .await?;
    fs::create_dir_all(context.settings.ingest.thumbnails_root.join("synctest02")).await?;
    fs::create_dir_all(context.settings.ingest.thumbnails_root.join("synctest05")).await?;

    repair_incomplete_ingest(&context.pool, &context.settings.ingest).await?;

    let rows = sqlx::query!(
        r#"
        SELECT relative_path, job_type::text AS "job_type!"
        FROM jobs
        WHERE relative_path IN ($1, $2, $3, $4, $5)
        "#,
        thumbs_path,
        analysis_path,
        missing_path,
        blocked_thumbs_path,
        blocked_analysis_path
    )
    .fetch_all(&context.pool)
    .await?;
    let has = |path: &str, job_type: &str| {
        rows.iter()
            .any(|row| row.relative_path.as_deref() == Some(path) && row.job_type == job_type)
    };

    assert!(has(thumbs_path, "ingest_thumbnails"));
    assert!(has(thumbs_path, "ingest_analysis"));
    assert!(!has(thumbs_path, "ingest_metadata"));
    assert!(has(analysis_path, "ingest_analysis"));
    assert!(!has(analysis_path, "ingest_thumbnails"));
    assert!(
        !rows
            .iter()
            .any(|row| row.relative_path.as_deref() == Some(missing_path))
    );
    assert!(!has(blocked_thumbs_path, "ingest_thumbnails"));
    assert!(!has(blocked_thumbs_path, "ingest_analysis"));
    assert!(!has(blocked_analysis_path, "ingest_analysis"));

    sqlx::query!(
        "DELETE FROM jobs WHERE relative_path IN ($1, $2, $3, $4, $5)",
        thumbs_path,
        analysis_path,
        missing_path,
        blocked_thumbs_path,
        blocked_analysis_path
    )
    .execute(&context.pool)
    .await?;
    sqlx::query!(
        "DELETE FROM media_item WHERE id IN \
         ('synctest01', 'synctest02', 'synctest03', 'synctest04', 'synctest05')"
    )
    .execute(&context.pool)
    .await?;
    let _ = fs::remove_file(context.settings.ingest.media_root.join(thumbs_path)).await;
    let _ = fs::remove_file(context.settings.ingest.media_root.join(analysis_path)).await;
    let _ = fs::remove_file(context.settings.ingest.media_root.join(blocked_thumbs_path)).await;
    let _ = fs::remove_file(
        context
            .settings
            .ingest
            .media_root
            .join(blocked_analysis_path),
    )
    .await;
    let _ = fs::remove_dir_all(context.settings.ingest.thumbnails_root.join("synctest01")).await;
    let _ = fs::remove_dir_all(context.settings.ingest.thumbnails_root.join("synctest02")).await;
    let _ = fs::remove_dir_all(context.settings.ingest.thumbnails_root.join("synctest04")).await;
    let _ = fs::remove_dir_all(context.settings.ingest.thumbnails_root.join("synctest05")).await;

    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn test_permanently_failed_ingest_blocks_repair(context: &TestContext) -> Result<()> {
    let user_id: i32 = sqlx::query_scalar!("SELECT id FROM app_user WHERE email = $1", EMAIL)
        .fetch_one(&context.pool)
        .await?;
    let template = sqlx::query!(
        "SELECT id, relative_path FROM media_item WHERE user_id = $1 AND is_video = false LIMIT 1",
        user_id
    )
    .fetch_one(&context.pool)
    .await?;
    let relative_path = "sync-ingest-test-failed.jpg";
    let source = context
        .settings
        .ingest
        .media_root
        .join(&template.relative_path);

    insert_incomplete_media_item(
        &context.pool,
        &template.id,
        "synctest06",
        relative_path,
        false,
    )
    .await?;
    fs::copy(
        &source,
        context.settings.ingest.media_root.join(relative_path),
    )
    .await?;

    let job_id: i64 = sqlx::query_scalar!(
        r#"
        INSERT INTO jobs (relative_path, user_id, job_type, scope, phase, priority, status,
                          owner, started_at, last_heartbeat, scheduled_at, attempts, max_attempts)
        VALUES ($1, $2, 'ingest_thumbnails'::job_type, 'path'::job_scope, 30, 60,
                'running'::job_status, 'repair-test', now(), now() + interval '1 hour',
                now() + interval '1 hour', 1, 2)
        RETURNING id
        "#,
        relative_path,
        user_id
    )
    .fetch_one(&context.pool)
    .await?;
    let job = Job {
        id: job_id,
        payload: None,
        relative_path: Some(relative_path.to_owned()),
        user_id: Some(user_id),
        job_type: JobType::IngestThumbnails,
        scope: JobScope::Path,
        phase: 30,
        priority: 60,
        attempts: 1,
        max_attempts: 2,
        dependency_attempts: 0,
    };

    update_job_on_failure(
        &context.pool,
        &job,
        &color_eyre::eyre::eyre!("simulated corrupt file"),
    )
    .await?;
    let blocked: bool = sqlx::query_scalar!(
        "SELECT thumbnail_repair_blocked FROM media_item WHERE id = 'synctest06'"
    )
    .fetch_one(&context.pool)
    .await?;
    assert!(blocked);

    let max_id: i64 = sqlx::query_scalar!(r#"SELECT COALESCE(MAX(id), 0) AS "max_id!" FROM jobs"#)
        .fetch_one(&context.pool)
        .await?;
    repair_incomplete_ingest(&context.pool, &context.settings.ingest).await?;
    let new_jobs: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*)::bigint AS "count!" FROM jobs
           WHERE id > $1 AND relative_path = $2"#,
        max_id,
        relative_path
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(new_jobs, 0);

    update_job_on_completion(&context.pool, &job, JobResult::Done).await?;
    let blocked: bool = sqlx::query_scalar!(
        "SELECT thumbnail_repair_blocked FROM media_item WHERE id = 'synctest06'"
    )
    .fetch_one(&context.pool)
    .await?;
    assert!(!blocked);

    let max_id: i64 = sqlx::query_scalar!(r#"SELECT COALESCE(MAX(id), 0) AS "max_id!" FROM jobs"#)
        .fetch_one(&context.pool)
        .await?;
    repair_incomplete_ingest(&context.pool, &context.settings.ingest).await?;
    let repaired: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*)::bigint AS "count!" FROM jobs
           WHERE id > $1 AND relative_path = $2 AND job_type = 'ingest_thumbnails'"#,
        max_id,
        relative_path
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(repaired, 1);

    // A job cancelled while running must not create a repair block when it later errors.
    sqlx::query!(
        "UPDATE jobs SET status = 'cancelled'::job_status, attempts = 1 WHERE id = $1",
        job_id
    )
    .execute(&context.pool)
    .await?;
    update_job_on_failure(
        &context.pool,
        &job,
        &color_eyre::eyre::eyre!("cancelled job then failed"),
    )
    .await?;
    let blocked: bool = sqlx::query_scalar!(
        "SELECT thumbnail_repair_blocked FROM media_item WHERE id = 'synctest06'"
    )
    .fetch_one(&context.pool)
    .await?;
    assert!(!blocked);

    // A cancelled job finishing must not clear an existing repair block.
    sqlx::query!("UPDATE media_item SET thumbnail_repair_blocked = true WHERE id = 'synctest06'")
        .execute(&context.pool)
        .await?;
    update_job_on_completion(&context.pool, &job, JobResult::Done).await?;
    let blocked: bool = sqlx::query_scalar!(
        "SELECT thumbnail_repair_blocked FROM media_item WHERE id = 'synctest06'"
    )
    .fetch_one(&context.pool)
    .await?;
    assert!(blocked);

    sqlx::query!("DELETE FROM jobs WHERE relative_path = $1", relative_path)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = 'synctest06'")
        .execute(&context.pool)
        .await?;
    let _ = fs::remove_file(context.settings.ingest.media_root.join(relative_path)).await;
    let _ = fs::remove_dir_all(context.settings.ingest.thumbnails_root.join("synctest06")).await;

    Ok(())
}
