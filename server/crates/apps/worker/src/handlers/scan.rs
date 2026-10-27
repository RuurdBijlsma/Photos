use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::clean_thumbnails::get_thumbnail_folders;
use crate::handlers::common::utils::require_media_mounted_fresh;
use app_state::{IngestSettings, MakeRelativePath};
use color_eyre::eyre::Result;
use common_services::database::jobs::{Job, JobType};
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::{bulk_enqueue_full_ingest, enqueue_job};
use common_services::media_mount::invalidate_mount_cache;
use common_services::media_reconcile;
use sqlx::PgPool;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;
use tracing::{info, warn};
use walkdir::WalkDir;

/// Checks if a file path has an extension present in a given set of allowed extensions.
fn has_allowed_ext(path: &Path, allowed: &HashSet<&str>) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| allowed.contains(ext.to_lowercase().as_str()))
}

/// Recursively finds all non-empty media files in a folder that have an allowed extension.
fn get_media_files(folder: &Path, allowed_exts: &HashSet<&str>) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(folder).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && entry.metadata().is_ok_and(|m| m.len() > 0)
            && has_allowed_ext(entry.path(), allowed_exts)
        {
            files.push(entry.into_path());
        }
    }
    files
}

/// Synchronizes the filesystem state with the database.
#[allow(clippy::too_many_lines)]
pub async fn sync_user_files_to_db(
    pool: &PgPool,
    settings: &IngestSettings,
    user_folder: &Path,
    user_id: i32,
) -> Result<()> {
    if !user_folder.exists() || !user_folder.is_dir() {
        warn!(
            "User folder {:?} does not exist or is not a directory. Skipping sync for user {}",
            user_folder, user_id
        );
        return Ok(());
    }

    let detection = &settings.file_detection;
    let allowed: HashSet<_> = detection
        .photo_extensions
        .iter()
        .chain(detection.video_extensions.iter())
        .map(String::as_str)
        .collect();

    let all_files = get_media_files(user_folder, &allowed);
    let other_media_folders = sqlx::query_scalar!(
        r#"
            SELECT media_folder as "media_folder!"
            FROM app_user
            WHERE id != $1 AND media_folder IS NOT NULL
            "#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    let user_rel_path = user_folder.make_relative(&settings.media_root)?;
    let sub_folders = other_media_folders
        .iter()
        .filter(|f| f.starts_with(&user_rel_path))
        .collect::<Vec<&String>>();
    let fs_paths: HashSet<String> = all_files
        .into_iter()
        .flat_map(|p| p.make_relative(&settings.media_root))
        // Filter out all fs_paths that start with another user's media_folder
        .filter(|fs_path| {
            sub_folders
                .iter()
                .all(|sub_folder| !fs_path.starts_with(*sub_folder))
        })
        .collect();

    let db_rows = sqlx::query!(
        r#"
        SELECT relative_path, deleted, (missing_since IS NOT NULL) AS "is_missing!"
        FROM media_item
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let mut db_all_paths = HashSet::with_capacity(db_rows.len());
    let mut db_present_paths = HashSet::new();
    let mut db_missing_paths = HashSet::new();

    for row in db_rows {
        if !row.deleted {
            if row.is_missing {
                db_missing_paths.insert(row.relative_path.clone());
            } else {
                db_present_paths.insert(row.relative_path.clone());
            }
        }
        db_all_paths.insert(row.relative_path);
    }

    // Safety guard: if filesystem has no files but the DB has records, the mount is likely disconnected
    if fs_paths.is_empty() && !db_all_paths.is_empty() {
        warn!(
            "User folder {:?} contains 0 media files while {} items exist in DB. Storage may be unmounted. Aborting scan for user {}.",
            user_folder,
            db_all_paths.len(),
            user_id
        );
        invalidate_mount_cache(&settings.media_root).await;
        return Ok(());
    }

    // New files on disk -> Enqueue ingest
    let to_ingest: Vec<_> = fs_paths.difference(&db_all_paths).cloned().collect();

    // WalkDir can skip permission errors, so verify every candidate path directly.
    let mut to_unmark_missing: Vec<_> = db_missing_paths.intersection(&fs_paths).cloned().collect();
    let mut candidates: Vec<_> = db_present_paths
        .difference(&fs_paths)
        .chain(db_missing_paths.difference(&fs_paths))
        .cloned()
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    let mut verified_missing = Vec::new();
    for path in candidates {
        match tokio::fs::metadata(settings.media_root.join(&path)).await {
            Ok(_) => {
                if db_missing_paths.contains(&path) {
                    to_unmark_missing.push(path);
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound => verified_missing.push(path),
            Err(e) => info!("Skipping {path} during scan: {e}"),
        }
    }

    let has_new_files = !to_ingest.is_empty();
    if has_new_files {
        info!(
            "Enqueuing {} new files for user {}",
            to_ingest.len(),
            user_id
        );
        let items: Vec<_> = to_ingest.into_iter().map(|path| (path, None)).collect();
        bulk_enqueue_full_ingest(pool, settings, &items, user_id).await?;
    }

    if !to_unmark_missing.is_empty() {
        info!(
            "Restoring {} reconnected files for user {}",
            to_unmark_missing.len(),
            user_id
        );
        MediaItemStore::unmark_relative_paths_as_missing(pool, &to_unmark_missing).await?;
    }

    if !verified_missing.is_empty() {
        info!(
            "Marking {} verified missing files for user {}",
            verified_missing.len(),
            user_id
        );
        let result =
            media_reconcile::reconcile_removed_paths(pool, settings, &verified_missing).await?;
        info!(
            "Reconciled missing paths: {} marked missing, {} job(s) cancelled",
            result.marked_missing, result.cancelled_jobs
        );
    }

    let repaired = repair_incomplete_stages(pool, settings, user_id).await?;
    if has_new_files || !to_unmark_missing.is_empty() || repaired > 0 {
        tokio::try_join!(
            enqueue_job::<()>(pool, settings, JobType::UpdateGlobalCentroid).call(),
            enqueue_job::<()>(pool, settings, JobType::ClusterFaces).call(),
            enqueue_job::<()>(pool, settings, JobType::ClusterPhotos).call(),
            enqueue_job::<()>(pool, settings, JobType::GenerateDailyCards).call(),
            enqueue_job::<()>(pool, settings, JobType::CalcSystemStats).call(),
        )?;
    }
    Ok(())
}

/// Enqueues the missing stage jobs for non-deleted, present items of one user.
///
/// Returns the number of items that needed repair, regardless of deduplication.
pub async fn repair_incomplete_stages(
    pool: &PgPool,
    settings: &IngestSettings,
    user_id: i32,
) -> Result<usize> {
    let rows = sqlx::query!(
        r#"
        SELECT relative_path,
               thumbnails_status::text AS "thumbnails_status!",
               analysis_status::text AS "analysis_status!"
        FROM media_item
        WHERE user_id = $1
          AND deleted = false
          AND missing_since IS NULL
          AND (
              thumbnails_status = 'pending'
              OR (thumbnails_status = 'done' AND analysis_status = 'pending')
          )
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let count = rows.len();
    for row in rows {
        let job_type = if row.thumbnails_status == "pending" {
            JobType::IngestThumbnails
        } else {
            JobType::IngestAnalysis
        };
        enqueue_job::<()>(pool, settings, job_type)
            .relative_path(&row.relative_path)
            .user_id(user_id)
            .call()
            .await?;
    }
    Ok(count)
}

/// Resets `done` thumbnail stages whose folder is gone, unless the app-data root looks empty.
pub async fn reset_missing_thumbnail_statuses(
    pool: &PgPool,
    settings: &IngestSettings,
) -> Result<()> {
    let thumb_ids = get_thumbnail_folders(&settings.thumbnails_root).await?;
    let done_count: i64 = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) AS "count!"
        FROM media_item
        WHERE thumbnails_status = 'done' AND deleted = false
        "#
    )
    .fetch_one(pool)
    .await?;
    if thumb_ids.is_empty() && done_count > 0 {
        warn!(
            "Thumbnail root {:?} is empty while {} done item(s) exist; skipping drift reset.",
            settings.thumbnails_root, done_count
        );
        return Ok(());
    }

    let ids: Vec<String> = thumb_ids.into_iter().collect();
    let result = sqlx::query!(
        r#"
        UPDATE media_item
        SET thumbnails_status = 'pending',
            thumbnails_error = NULL,
            updated_at = now()
        WHERE thumbnails_status = 'done'
          AND deleted = false
          AND NOT (id = ANY($1))
        "#,
        &ids
    )
    .execute(pool)
    .await?;
    if result.rows_affected() > 0 {
        info!(
            "Reset {} thumbnail stage(s) with missing folders.",
            result.rows_affected()
        );
    }
    Ok(())
}

/// Run the indexing scan across all users, or only the requested user.
pub async fn run_scan(
    pool: &PgPool,
    settings: &IngestSettings,
    user_id: Option<i32>,
) -> Result<()> {
    if user_id.is_none() {
        reset_missing_thumbnail_statuses(pool, settings).await?;
    }
    let users = UserStore::list_users_with_media_folders(pool).await?;
    let media_root = &settings.media_root;
    info!("Scanning \"{}\" ...", &media_root.display());
    for user in users {
        if user_id.is_some_and(|requested| requested != user.id) {
            continue;
        }
        let Some(media_folder) = user.media_folder else {
            continue;
        };
        sync_user_files_to_db(pool, settings, &media_root.join(media_folder), user.id).await?;
    }
    info!("User scan complete");

    Ok(())
}

/// Triggers a full scan to synchronise the filesystem and database.
pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    if let Some(result) =
        require_media_mounted_fresh(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }
    run_scan(&context.pool, &context.settings.ingest, job.user_id).await?;
    Ok(JobResult::Done)
}
