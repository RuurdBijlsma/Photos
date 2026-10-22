use crate::runner::context::test_context::TestContext;
use crate::test_helpers::media_dir_contents;
use color_eyre::Result;
use color_eyre::eyre::{bail, eyre};
use common_services::database::jobs::JobType;
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
