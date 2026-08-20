use app_state::{IngestSettings, MakeRelativePath};
use color_eyre::eyre::eyre;
use common_services::database::user_store::UserStore;
use common_services::job_queue::enqueue_full_ingest;
use sqlx::PgPool;
use std::path::Path;
use tracing::{info};
use walkdir::WalkDir;

/// Handles a create event from the watcher.
pub async fn handle_create(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<()> {
    if path.is_file() {
        enqueue_ingest_job(pool, settings, path).await?;
    } else {
        info!("Directory created: {:?}. Scanning for new files.", path);
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                enqueue_ingest_job(pool, settings, entry.path()).await?;
            }
        }
    }
    Ok(())
}

/// A helper function to enqueue a job for a given file path.
async fn enqueue_ingest_job(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<()> {
    let relative_path = &path.make_relative(&settings.media_root)?;
    let user = UserStore::find_user_by_relative_path(pool, relative_path)
        .await?
        .ok_or_else(|| {
            eyre!(
                "Could not find user for path: {}, cannot enqueue job.",
                relative_path
            )
        })?;

    enqueue_full_ingest(pool, settings, relative_path, user.id, None).await?;

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
