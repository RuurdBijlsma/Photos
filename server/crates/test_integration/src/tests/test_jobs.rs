use crate::runner::context::test_context::TestContext;
use crate::test_helpers::media_dir_contents;
use color_eyre::Result;
use color_eyre::eyre::{bail, eyre};
use common_services::database::jobs::{Job, JobType};
use common_services::database::media_item::media_item::CreateFullMediaItem;
use common_services::database::media_item_store::MediaItemStore;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[allow(dead_code)]
pub(crate) async fn insert_media_item_copy(
    pool: &PgPool,
    source_id: &str,
    new_id: &str,
    new_path: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO media_item (
            id, user_id, remote_user_id, hash, relative_path, filename,
            width, height, is_video, duration_ms, orientation,
            taken_at_local, og_taken_at_local, taken_at_utc, sort_timestamp,
            timezone_name, timezone_offset_seconds, og_timezone_offset_seconds,
            use_panorama_viewer, deleted, missing_since
        )
        SELECT $2, user_id, remote_user_id, hash, $3, regexp_replace($3, '^.*/', ''),
               width, height, is_video, duration_ms, orientation,
               taken_at_local, og_taken_at_local, taken_at_utc, sort_timestamp,
               timezone_name, timezone_offset_seconds, og_timezone_offset_seconds,
               use_panorama_viewer, false, NULL
        FROM media_item WHERE id = $1
        "#,
        source_id,
        new_id,
        new_path
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) async fn insert_job(
    pool: &PgPool,
    user_id: i32,
    relative_path: &str,
    job_type: JobType,
    status: &str,
    attempts: i32,
    max_attempts: i32,
) -> Result<i64> {
    let phase = job_type.phase();
    let job_type = serde_json::to_value(job_type)?;
    let job_type = job_type.as_str().expect("job type serializes to a string");
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO jobs (
            relative_path, user_id, job_type, scope, phase, priority,
            status, attempts, max_attempts, scheduled_at
        )
        VALUES ($1, $2, $3::text::job_type, 'path'::job_scope, $4, 50,
                $5::text::job_status, $6, $7, now() + interval '1 hour')
        RETURNING id
        "#,
        relative_path,
        user_id,
        job_type,
        phase,
        status,
        attempts,
        max_attempts
    )
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn test_generated_thumbnails_status(context: &TestContext) -> Result<()> {
    let mut tx = context.pool.begin().await?;
    let id: String = sqlx::query_scalar!("SELECT id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query!(
        "UPDATE media_item SET thumbnails_status = 'done' WHERE id = $1",
        &id
    )
    .execute(&mut *tx)
    .await?;
    let has: bool = sqlx::query_scalar!(
        r#"SELECT has_thumbnails AS "has_thumbnails!" FROM media_item WHERE id = $1"#,
        &id
    )
    .fetch_one(&mut *tx)
    .await?;
    assert!(has);

    sqlx::query!(
        "UPDATE media_item SET thumbnails_status = 'pending' WHERE id = $1",
        &id
    )
    .execute(&mut *tx)
    .await?;
    let has: bool = sqlx::query_scalar!(
        r#"SELECT has_thumbnails AS "has_thumbnails!" FROM media_item WHERE id = $1"#,
        &id
    )
    .fetch_one(&mut *tx)
    .await?;
    assert!(!has);

    tx.rollback().await?;
    Ok(())
}

pub async fn test_update_full_resets_ingest_stages(context: &TestContext) -> Result<()> {
    let row = sqlx::query!(
        r#"
        SELECT id, relative_path
        FROM media_item mi
        WHERE mi.deleted = false
          AND mi.is_video = false
          AND mi.has_thumbnails = true
          AND EXISTS (
              SELECT 1 FROM visual_analysis va WHERE va.media_item_id = mi.id
          )
        LIMIT 1
        "#
    )
    .fetch_one(&context.pool)
    .await?;

    let file_path = context.settings.ingest.media_root.join(&row.relative_path);
    let analyzer = media_analyzer::MediaAnalyzer::builder().build().await?;
    let metadata = analyzer.analyze_media(&file_path).await?;
    let create_item: CreateFullMediaItem = metadata.into();

    let mut tx = context.pool.begin().await?;
    let analysis_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM visual_analysis WHERE media_item_id = $1",
        &row.id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);
    assert!(
        analysis_count > 0,
        "onboarding must produce analysis before this test"
    );

    MediaItemStore::update_full(&mut tx, &row.id, &create_item).await?;

    let after = sqlx::query!(
        r#"
        SELECT thumbnails_status::text AS "thumbnails_status!",
               analysis_status::text AS "analysis_status!",
               has_thumbnails AS "has_thumbnails!",
               (missing_since IS NULL) AS "not_missing!"
        FROM media_item WHERE id = $1
        "#,
        &row.id
    )
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(after.thumbnails_status, "pending");
    assert_eq!(after.analysis_status, "pending");
    assert!(!after.has_thumbnails);
    assert!(after.not_missing);

    let analysis_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM visual_analysis WHERE media_item_id = $1",
        &row.id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);
    assert_eq!(analysis_count, 0);

    tx.rollback().await?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn test_media_reconcile_removal(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT id, user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let ids = vec![
        "recon-1".to_owned(),
        "recon-2".to_owned(),
        "recon-3".to_owned(),
    ];
    insert_media_item_copy(&context.pool, &source.id, &ids[0], "recon-test/file.jpg").await?;
    insert_media_item_copy(
        &context.pool,
        &source.id,
        &ids[1],
        "recon-test/dir/child.jpg",
    )
    .await?;
    insert_media_item_copy(&context.pool, &source.id, &ids[2], "recon-test/keep.jpg").await?;

    let jobs = vec![
        insert_job(
            &context.pool,
            source.user_id,
            "recon-test/file.jpg",
            JobType::IngestMetadata,
            "queued",
            0,
            3,
        )
        .await?,
        insert_job(
            &context.pool,
            source.user_id,
            "recon-test/dir/child.jpg",
            JobType::IngestMetadata,
            "queued",
            0,
            3,
        )
        .await?,
        insert_job(
            &context.pool,
            source.user_id,
            "recon-test/keep.jpg",
            JobType::IngestMetadata,
            "queued",
            0,
            3,
        )
        .await?,
    ];

    let result = common_services::media_reconcile::reconcile_removed_paths(
        &context.pool,
        &context.settings.ingest,
        &[
            "recon-test/file.jpg".to_owned(),
            "recon-test/dir".to_owned(),
        ],
    )
    .await?;
    assert_eq!(result.marked_missing, 2);
    assert_eq!(result.cancelled_jobs, 2);

    let rows = sqlx::query!(
        r#"
        SELECT id, relative_path, (missing_since IS NOT NULL) AS "missing!"
        FROM media_item
        WHERE id = ANY($1)
        ORDER BY id
        "#,
        &ids
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(rows.len(), 3);
    for row in rows {
        let missing = row.relative_path != "recon-test/keep.jpg";
        assert_eq!(row.missing, missing, "{}", row.relative_path);
    }

    let statuses = sqlx::query!(
        r#"
        SELECT id, status::text AS "status!"
        FROM jobs
        WHERE id = ANY($1)
        ORDER BY id
        "#,
        &jobs
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(statuses.len(), 3);
    for row in statuses {
        let expected = if row.id == jobs[2] {
            "queued"
        } else {
            "cancelled"
        };
        assert_eq!(row.status, expected, "job {}", row.id);
    }

    sqlx::query!("DELETE FROM jobs WHERE id = ANY($1)", &jobs)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = ANY($1)", &ids)
        .execute(&context.pool)
        .await?;
    Ok(())
}

fn retry_failure_job(job_id: i64, relative_path: &str, user_id: i32, job_type: JobType) -> Job {
    Job {
        id: job_id,
        payload: None,
        relative_path: Some(relative_path.to_owned()),
        user_id: Some(user_id),
        job_type,
        scope: job_type.scope(),
        phase: job_type.phase(),
        priority: job_type.get_priority(false),
        attempts: 4,
        max_attempts: 5,
        dependency_attempts: 0,
    }
}

pub async fn test_bulk_enqueue_sorts_paths(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let paths = vec![
        "bulk-sort-test/z.jpg".to_owned(),
        "bulk-sort-test/a.jpg".to_owned(),
        "bulk-sort-test/m.jpg".to_owned(),
    ];
    let items: Vec<_> = paths.iter().map(|path| (path.clone(), None)).collect();
    common_services::job_queue::bulk_enqueue_full_ingest(
        &context.pool,
        &context.settings.ingest,
        &items,
        source.user_id,
    )
    .await?;

    let rows = sqlx::query!(
        r#"
        SELECT relative_path
        FROM jobs
        WHERE relative_path = ANY($1) AND job_type = 'ingest_metadata'
        ORDER BY id
        "#,
        &paths
    )
    .fetch_all(&context.pool)
    .await?;
    let inserted: Vec<_> = rows
        .into_iter()
        .filter_map(|row| row.relative_path)
        .collect();
    assert_eq!(
        inserted,
        vec![
            "bulk-sort-test/a.jpg".to_owned(),
            "bulk-sort-test/m.jpg".to_owned(),
            "bulk-sort-test/z.jpg".to_owned()
        ]
    );

    sqlx::query!("DELETE FROM jobs WHERE relative_path = ANY($1)", &paths)
        .execute(&context.pool)
        .await?;
    Ok(())
}

pub async fn test_stage_failure_and_retry(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT id, user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let path = "retry-test/thumb.jpg";
    let id = "retry-1";
    insert_media_item_copy(&context.pool, &source.id, id, path).await?;
    let job_id = insert_job(
        &context.pool,
        source.user_id,
        path,
        JobType::IngestThumbnails,
        "running",
        4,
        5,
    )
    .await?;
    sqlx::query!(
        "UPDATE jobs SET owner = 'test', started_at = now(), last_heartbeat = now() WHERE id = $1",
        job_id
    )
    .execute(&context.pool)
    .await?;
    let job = retry_failure_job(job_id, path, source.user_id, JobType::IngestThumbnails);

    worker::jobs::management::update_job_on_failure(&context.pool, &job, &eyre!("boom")).await?;

    let job_status: String = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM jobs WHERE id = $1"#,
        job_id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(job_status, "failed");
    let stage = sqlx::query!(
        r#"
        SELECT thumbnails_status::text AS "thumbnails_status!", thumbnails_error
        FROM media_item WHERE id = $1
        "#,
        id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(stage.thumbnails_status, "failed");
    assert!(stage.thumbnails_error.is_some());

    common_services::api::jobs::user_level::retry_user_job(&context.pool, job_id, source.user_id)
        .await?;
    let job_status: String = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM jobs WHERE id = $1"#,
        job_id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_ne!(job_status, "failed");
    let stage = sqlx::query!(
        r#"
        SELECT thumbnails_status::text AS "thumbnails_status!", thumbnails_error
        FROM media_item WHERE id = $1
        "#,
        id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(stage.thumbnails_status, "pending");
    assert!(stage.thumbnails_error.is_none());

    sqlx::query!("DELETE FROM jobs WHERE id = $1", job_id)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = $1", id)
        .execute(&context.pool)
        .await?;
    Ok(())
}

pub async fn test_cancelled_job_not_overwritten(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT id, user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let path = "retry-test/cancelled.jpg";
    let id = "retry-2";
    insert_media_item_copy(&context.pool, &source.id, id, path).await?;
    let job_id = insert_job(
        &context.pool,
        source.user_id,
        path,
        JobType::IngestThumbnails,
        "running",
        4,
        5,
    )
    .await?;
    sqlx::query!(
        "UPDATE jobs SET status = 'cancelled', owner = 'test', started_at = now() WHERE id = $1",
        job_id
    )
    .execute(&context.pool)
    .await?;
    let job = retry_failure_job(job_id, path, source.user_id, JobType::IngestThumbnails);

    worker::jobs::management::update_job_on_failure(&context.pool, &job, &eyre!("boom")).await?;
    let job_status: String = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM jobs WHERE id = $1"#,
        job_id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(job_status, "cancelled");
    let stage: String = sqlx::query_scalar!(
        r#"SELECT thumbnails_status::text AS "stage!" FROM media_item WHERE id = $1"#,
        id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(stage, "pending");

    worker::jobs::management::update_job_on_completion(
        &context.pool,
        &job,
        worker::handlers::JobResult::Done,
    )
    .await?;
    let job_status: String = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM jobs WHERE id = $1"#,
        job_id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(job_status, "cancelled");

    sqlx::query!("DELETE FROM jobs WHERE id = $1", job_id)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = $1", id)
        .execute(&context.pool)
        .await?;
    Ok(())
}

pub async fn test_scan_repairs_incomplete_items(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT id, user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let ids = vec![
        "repair-1".to_owned(),
        "repair-2".to_owned(),
        "repair-3".to_owned(),
        "repair-4".to_owned(),
    ];
    let paths = vec![
        "repair-test/thumbs.jpg".to_owned(),
        "repair-test/analysis.jpg".to_owned(),
        "repair-test/failed.jpg".to_owned(),
        "repair-test/done.jpg".to_owned(),
    ];
    for ((id, path), statuses) in ids.iter().zip(&paths).zip([
        ("pending", "pending"),
        ("done", "pending"),
        ("done", "failed"),
        ("done", "done"),
    ]) {
        insert_media_item_copy(&context.pool, &source.id, id, path).await?;
        sqlx::query!(
            "UPDATE media_item SET thumbnails_status = $2::text::ingest_stage_status, analysis_status = $3::text::ingest_stage_status WHERE id = $1",
            id,
            statuses.0,
            statuses.1
        )
        .execute(&context.pool)
        .await?;
    }
    let baseline: i64 = sqlx::query_scalar!(r#"SELECT COALESCE(MAX(id), 0) AS "max!" FROM jobs"#)
        .fetch_one(&context.pool)
        .await?;

    let repaired = worker::handlers::scan::repair_incomplete_stages(
        &context.pool,
        &context.settings.ingest,
        source.user_id,
    )
    .await?;
    assert!(repaired >= 2);

    let rows = sqlx::query!(
        r#"
        SELECT relative_path, job_type::text AS "job_type!"
        FROM jobs
        WHERE id > $1 AND relative_path = ANY($2)
        ORDER BY id
        "#,
        baseline,
        &paths
    )
    .fetch_all(&context.pool)
    .await?;
    let mut jobs_by_path: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for row in rows {
        if let Some(path) = row.relative_path {
            jobs_by_path.entry(path).or_default().push(row.job_type);
        }
    }
    assert_eq!(
        jobs_by_path.get(&paths[0]),
        Some(&vec!["ingest_thumbnails".to_owned()])
    );
    assert_eq!(
        jobs_by_path.get(&paths[1]),
        Some(&vec!["ingest_analysis".to_owned()])
    );
    assert!(!jobs_by_path.contains_key(&paths[2]));
    assert!(!jobs_by_path.contains_key(&paths[3]));

    sqlx::query!("DELETE FROM jobs WHERE relative_path = ANY($1)", &paths)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = ANY($1)", &ids)
        .execute(&context.pool)
        .await?;
    Ok(())
}

pub async fn test_thumbnail_drift_reset(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT id, user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let id = "drift-1";
    insert_media_item_copy(&context.pool, &source.id, id, "drift-test/no-folder.jpg").await?;
    sqlx::query!(
        "UPDATE media_item SET thumbnails_status = 'done', analysis_status = 'done' WHERE id = $1",
        id
    )
    .execute(&context.pool)
    .await?;
    assert!(
        !context.settings.ingest.thumbnails_root.join(id).exists(),
        "drift test needs an item without a thumbnail folder"
    );

    let real_id: Option<String> = sqlx::query_scalar!(
        r#"
        SELECT id FROM media_item
        WHERE deleted = false AND thumbnails_status = 'done' AND id != $1
        "#,
        id
    )
    .fetch_all(&context.pool)
    .await?
    .into_iter()
    .find(|real_id| {
        context
            .settings
            .ingest
            .thumbnails_root
            .join(real_id)
            .is_dir()
    });

    worker::handlers::scan::reset_missing_thumbnail_statuses(
        &context.pool,
        &context.settings.ingest,
    )
    .await?;

    let after = sqlx::query!(
        r#"
        SELECT thumbnails_status::text AS "thumbnails_status!", thumbnails_error
        FROM media_item WHERE id = $1
        "#,
        id
    )
    .fetch_one(&context.pool)
    .await?;
    assert_eq!(after.thumbnails_status, "pending");
    assert!(after.thumbnails_error.is_none());

    if let Some(real_id) = real_id {
        let status: String = sqlx::query_scalar!(
            r#"SELECT thumbnails_status::text AS "status!" FROM media_item WHERE id = $1"#,
            &real_id
        )
        .fetch_one(&context.pool)
        .await?;
        assert_eq!(status, "done");
    }

    sqlx::query!("DELETE FROM media_item WHERE id = $1", id)
        .execute(&context.pool)
        .await?;
    Ok(())
}

pub async fn test_clean_thumbnail_orphans(context: &TestContext) -> Result<()> {
    let thumbnails_root = &context.settings.ingest.thumbnails_root;
    let orphan = thumbnails_root.join("orphan0001");
    tokio::fs::create_dir_all(&orphan).await?;
    tokio::fs::write(orphan.join("dummy.txt"), b"x").await?;

    let source = sqlx::query!("SELECT id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let real_id: Option<String> = sqlx::query_scalar!("SELECT id FROM media_item")
        .fetch_all(&context.pool)
        .await?
        .into_iter()
        .find(|id| thumbnails_root.join(id).is_dir());
    let created_real = real_id.is_none();
    let real_id = real_id.unwrap_or(source.id);
    let real_folder = thumbnails_root.join(&real_id);
    tokio::fs::create_dir_all(&real_folder).await?;
    let preserved = real_folder.join("preserve.txt");
    tokio::fs::write(&preserved, b"x").await?;

    worker::handlers::clean_thumbnails::clean_thumbnail_orphans(
        &context.pool,
        &context.settings.ingest,
    )
    .await?;

    assert!(!orphan.exists(), "orphan thumbnail folder must be removed");
    assert!(real_folder.exists(), "known thumbnail folder must remain");

    tokio::fs::remove_file(&preserved).await?;
    if created_real {
        tokio::fs::remove_dir_all(&real_folder).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
pub async fn test_prune_missing_ingest_jobs(context: &TestContext) -> Result<()> {
    let source = sqlx::query!("SELECT id, user_id FROM media_item WHERE deleted = false LIMIT 1")
        .fetch_one(&context.pool)
        .await?;
    let ids = vec![
        "sweep-1".to_owned(),
        "sweep-2".to_owned(),
        "sweep-3".to_owned(),
    ];
    insert_media_item_copy(&context.pool, &source.id, &ids[0], "sweep-test/missing.jpg").await?;
    insert_media_item_copy(
        &context.pool,
        &source.id,
        &ids[1],
        "sweep-test/soft-deleted.jpg",
    )
    .await?;
    insert_media_item_copy(
        &context.pool,
        &source.id,
        &ids[2],
        "import/sweep-test-present.jpg",
    )
    .await?;
    sqlx::query!(
        "UPDATE media_item SET deleted = true WHERE id = $1",
        &ids[1]
    )
    .execute(&context.pool)
    .await?;
    sqlx::query!(
        "UPDATE media_item SET missing_since = now() WHERE id = $1",
        &ids[2]
    )
    .execute(&context.pool)
    .await?;

    let present_path = context
        .settings
        .ingest
        .media_root
        .join("import/sweep-test-present.jpg");
    if let Some(parent) = present_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&present_path, b"x").await?;

    let jobs = vec![
        insert_job(
            &context.pool,
            source.user_id,
            "sweep-test/missing.jpg",
            JobType::IngestMetadata,
            "queued",
            0,
            3,
        )
        .await?,
        insert_job(
            &context.pool,
            source.user_id,
            "sweep-test/soft-deleted.jpg",
            JobType::IngestThumbnails,
            "queued",
            0,
            3,
        )
        .await?,
        insert_job(
            &context.pool,
            source.user_id,
            "import/sweep-test-present.jpg",
            JobType::IngestAnalysis,
            "queued",
            0,
            3,
        )
        .await?,
    ];

    let result = worker::handlers::prune_missing_ingest_jobs::prune_missing_ingest_jobs(
        &context.pool,
        &context.settings.ingest,
    )
    .await?;
    assert!(matches!(result, worker::handlers::JobResult::Done));

    let rows = sqlx::query!(
        r#"
        SELECT id, (missing_since IS NOT NULL) AS "missing!"
        FROM media_item
        WHERE id = ANY($1)
        ORDER BY id
        "#,
        &ids
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(rows.len(), 3);
    assert!(rows[0].missing, "missing file should be marked");
    assert!(!rows[2].missing, "present file should be restored");

    let statuses = sqlx::query!(
        r#"
        SELECT id, status::text AS "status!"
        FROM jobs
        WHERE id = ANY($1)
        ORDER BY id
        "#,
        &jobs
    )
    .fetch_all(&context.pool)
    .await?;
    assert_eq!(statuses.len(), 3);
    assert_eq!(statuses[0].status, "cancelled");
    assert_eq!(statuses[1].status, "cancelled");
    assert_eq!(statuses[2].status, "queued");

    sqlx::query!("DELETE FROM jobs WHERE id = ANY($1)", &jobs)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = ANY($1)", &ids)
        .execute(&context.pool)
        .await?;
    if present_path.exists() {
        tokio::fs::remove_file(&present_path).await?;
    }
    Ok(())
}

pub async fn test_watcher_delete(context: &TestContext) -> Result<()> {
    let (photos, _) = media_dir_contents(context)?;
    let source = photos
        .iter()
        .find(|path| !path.to_string_lossy().contains("watcher-e2e"))
        .or_else(|| photos.first())
        .ok_or_else(|| eyre!("no photo asset found for watcher test"))?;
    let media_folder: Option<String> =
        sqlx::query_scalar!("SELECT media_folder FROM app_user ORDER BY id LIMIT 1")
            .fetch_one(&context.pool)
            .await?;
    let folder = media_folder.unwrap_or_default();
    let prefix = if folder.is_empty() {
        "watcher-e2e".to_owned()
    } else {
        format!("{folder}/watcher-e2e")
    };
    let test_dir = context.settings.ingest.media_root.join(&prefix);
    tokio::fs::create_dir_all(&test_dir).await?;
    let relative_path = format!("{prefix}/delete-source.jpg");
    let path = context.settings.ingest.media_root.join(&relative_path);
    tokio::fs::copy(source, &path).await?;

    let start = Instant::now();
    let id = loop {
        let id: Option<String> = sqlx::query_scalar!(
            "SELECT id FROM media_item WHERE relative_path = $1",
            &relative_path
        )
        .fetch_optional(&context.pool)
        .await?;
        if let Some(id) = id {
            break id;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = tokio::fs::remove_file(&path).await;
            bail!("watcher did not create media item for {relative_path}");
        }
        sleep(Duration::from_millis(100)).await;
    };

    let start = Instant::now();
    loop {
        let ready: bool = sqlx::query_scalar!(
            r#"
            SELECT (
                EXISTS (
                    SELECT 1 FROM jobs
                    WHERE relative_path = $1
                      AND job_type = 'ingest_metadata'
                      AND status = 'done'
                )
                AND EXISTS (
                    SELECT 1 FROM jobs
                    WHERE relative_path = $1
                      AND job_type = 'ingest_thumbnails'
                      AND status IN ('done', 'cancelled', 'failed')
                )
            ) AS "ready!"
            "#,
            &relative_path
        )
        .fetch_one(&context.pool)
        .await?;
        if ready {
            break;
        }
        if start.elapsed() > Duration::from_secs(60) {
            let _ = tokio::fs::remove_file(&path).await;
            bail!("watcher media item was not fully ingested: {relative_path}");
        }
        sleep(Duration::from_millis(100)).await;
    }

    tokio::fs::remove_file(&path).await?;
    let start = Instant::now();
    loop {
        let missing: bool = sqlx::query_scalar!(
            r#"SELECT (missing_since IS NOT NULL) AS "missing!" FROM media_item WHERE id = $1"#,
            &id
        )
        .fetch_one(&context.pool)
        .await?;
        if missing {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            bail!("watcher did not mark {relative_path} missing after deletion");
        }
        sleep(Duration::from_millis(100)).await;
    }

    sqlx::query!("DELETE FROM jobs WHERE relative_path = $1", &relative_path)
        .execute(&context.pool)
        .await?;
    sqlx::query!("DELETE FROM media_item WHERE id = $1", &id)
        .execute(&context.pool)
        .await?;
    if test_dir.exists() {
        tokio::fs::remove_dir_all(&test_dir).await?;
    }
    Ok(())
}

pub async fn test_visual_analysis_unique_percentage(context: &TestContext) -> Result<()> {
    let mut tx = context.pool.begin().await?;
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM visual_analysis")
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
    assert!(
        count > 0,
        "onboarding must produce at least one analysis row"
    );

    // INSERT ... SELECT avoids binding a pgvector value in the test crate.
    let duplicate = sqlx::query!(
        r#"
        INSERT INTO visual_analysis (user_id, media_item_id, embedding, percentage)
        SELECT user_id, media_item_id, embedding, percentage
        FROM visual_analysis
        LIMIT 1
        "#
    )
    .execute(&mut *tx)
    .await;
    assert!(
        duplicate.is_err(),
        "duplicate (media_item_id, percentage) must fail"
    );

    tx.rollback().await?;
    Ok(())
}
