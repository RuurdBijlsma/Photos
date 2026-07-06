use app_state::{IngestSettings, MakeRelativePath};
use common_services::database::user_store::UserStore;
use common_services::job_queue::enqueue_full_ingest;
use sqlx::PgPool;
use std::path::Path;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

enum WatcherJobType {
    Ingest,
    Remove,
}

/// Handles a create event from the watcher.
pub async fn handle_create(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<()> {
    if path.is_file() {
        enqueue_file_job(pool, settings, path, WatcherJobType::Ingest).await?;
    } else {
        info!("Directory created: {:?}. Scanning for new files.", path);
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                enqueue_file_job(pool, settings, entry.path(), WatcherJobType::Ingest).await?;
            }
        }
    }
    Ok(())
}

/// Handles a remove event from the watcher.
pub async fn handle_remove(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<()> {
    // This logic is preserved as per your request to differentiate file from folder deletes.
    if is_path_in_db(pool, settings, path).await? {
        enqueue_file_job(pool, settings, path, WatcherJobType::Remove).await?;
    } else {
        info!(
            "Directory removed: {:?}. Removing all media items within.",
            path
        );
        handle_remove_folder(pool, settings, path).await?;
    }
    Ok(())
}

/// A helper function to enqueue a job for a given file path.
async fn enqueue_file_job(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
    job_type: WatcherJobType,
) -> color_eyre::Result<()> {
    let relative_path = &path.make_relative(&settings.media_root)?;
    let Some(user) = UserStore::find_user_by_relative_path(pool, relative_path).await? else {
        warn!(
            "Could not find user for path: {}. Cannot enqueue job.",
            relative_path
        );
        return Ok(());
    };

    match job_type {
        WatcherJobType::Ingest => {
            enqueue_full_ingest(pool, settings, relative_path, user.id, None).await?;
        }
        WatcherJobType::Remove => {
            common_services::api::photos::removal::delete_item_and_thumbnails(
                pool,
                &settings.thumbnails_root,
                relative_path,
            )
            .await?;
        }
    }

    Ok(())
}

/// Handles removing a folder by finding all its items in the DB and enqueuing their removal.
async fn handle_remove_folder(
    pool: &PgPool,
    settings: &IngestSettings,
    folder: &Path,
) -> color_eyre::Result<()> {
    let relative_dir = folder.make_relative(&settings.media_root)?;
    let pattern = format!("{relative_dir}%");

    let relative_paths = sqlx::query_scalar!(
        r"SELECT relative_path FROM media_item WHERE relative_path LIKE $1",
        pattern
    )
    .fetch_all(pool)
    .await?;

    if relative_paths.is_empty() {
        debug!(
            "No media items found in DB for removed directory: {:?}",
            folder
        );
        return Ok(());
    }

    for path in relative_paths {
        let absolute_path = settings.media_root.join(path);
        enqueue_file_job(pool, settings, &absolute_path, WatcherJobType::Remove).await?;
    }

    Ok(())
}

/// Checks if a given path exists in either the `media_item` or `jobs` table.
async fn is_path_in_db(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<bool> {
    let relative_path = path.make_relative(&settings.media_root)?;
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM media_item WHERE relative_path = $1
            UNION ALL
            SELECT 1 FROM jobs WHERE relative_path = $1
        )
        "#,
        relative_path
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);

    Ok(exists)
}
