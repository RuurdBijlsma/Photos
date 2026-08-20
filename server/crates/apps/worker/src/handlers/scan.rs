use crate::context::WorkerContext;
use crate::handlers::JobResult;
use app_state::{IngestSettings, MakeRelativePath};
use color_eyre::eyre::Result;
use common_services::database::jobs::{Job, JobType};
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::{bulk_enqueue_full_ingest, enqueue_job};
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;
use tracing::{info, warn};
use walkdir::WalkDir;

/// Checks if a file path has an extension present in a given set of allowed extensions.
fn has_allowed_ext(path: &Path, allowed: &HashSet<&str>) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| allowed.contains(ext.to_lowercase().as_str()))
}

/// Recursively finds all media files in a folder that have an allowed extension.
fn get_media_files(folder: &Path, allowed_exts: &HashSet<&str>) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(folder).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && has_allowed_ext(entry.path(), allowed_exts) {
            files.push(entry.into_path());
        }
    }
    files
}

/// Synchronizes the filesystem state with the database.
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
        SELECT relative_path, (missing_since IS NOT NULL) AS "is_missing!"
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
        if row.is_missing {
            db_missing_paths.insert(row.relative_path.clone());
        } else {
            db_present_paths.insert(row.relative_path.clone());
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
        return Ok(());
    }

    // New files on disk -> Enqueue ingest
    let to_ingest: Vec<_> = fs_paths.difference(&db_all_paths).cloned().collect();

    // Previously present files that are now missing -> Set missing_since = NOW()
    let to_mark_missing: Vec<_> = db_present_paths.difference(&fs_paths).cloned().collect();

    // Reconnected files that were marked missing -> Set missing_since = NULL
    let to_unmark_missing: Vec<_> = db_missing_paths.intersection(&fs_paths).cloned().collect();

    if !to_ingest.is_empty() {
        info!(
            "Enqueuing {} new files for user {}",
            to_ingest.len(),
            user_id
        );
        bulk_enqueue_full_ingest(pool, settings, &to_ingest, user_id).await?;
    }

    if !to_unmark_missing.is_empty() {
        info!(
            "Restoring {} reconnected files for user {}",
            to_unmark_missing.len(),
            user_id
        );
        MediaItemStore::unmark_relative_paths_as_missing(pool, &to_unmark_missing).await?;
    }

    if !to_mark_missing.is_empty() {
        info!(
            "Marking {} missing files for user {}",
            to_mark_missing.len(),
            user_id
        );
        MediaItemStore::mark_relative_paths_as_missing(pool, &to_mark_missing).await?;
    }

    if !to_ingest.is_empty() || !to_unmark_missing.is_empty() {
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

/// Run the indexing scan across all users.
pub async fn run_scan(pool: &PgPool, settings: &IngestSettings) -> Result<()> {
    let users = UserStore::list_users_with_media_folders(pool).await?;
    let media_root = &settings.media_root;
    info!("Scanning \"{}\" ...", &media_root.display());
    for user in users {
        let Some(media_folder) = user.media_folder else {
            continue;
        };
        sync_user_files_to_db(pool, settings, &media_root.join(media_folder), user.id).await?;
    }
    info!("User scan complete");

    Ok(())
}

/// Triggers a full scan to synchronise the filesystem and database.
pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    run_scan(&context.pool, &context.settings.ingest).await?;
    Ok(JobResult::Done)
}
