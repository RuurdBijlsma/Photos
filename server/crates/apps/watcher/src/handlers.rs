use app_state::{IngestSettings, MakeRelativePath};
use color_eyre::eyre::eyre;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::enqueue_full_ingest;
use sqlx::PgPool;
use std::path::Path;
use tracing::info;
use walkdir::WalkDir;

/// Handles a create event from the watcher.
pub async fn handle_create(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<()> {
    if path.is_file() {
        handle_file_create(pool, settings, path).await?;
    } else {
        info!("Directory created: {:?}. Scanning for new files.", path);
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                handle_file_create(pool, settings, entry.path()).await?;
            }
        }
    }
    Ok(())
}

async fn handle_file_create(
    pool: &PgPool,
    settings: &IngestSettings,
    path: &Path,
) -> color_eyre::Result<()> {
    let relative_path = path.make_relative(&settings.media_root)?;
    let user = UserStore::find_user_by_relative_path(pool, &relative_path)
        .await?
        .ok_or_else(|| {
            eyre!(
                "Could not find user for path: {}, cannot enqueue job.",
                relative_path
            )
        })?;

    let existing = sqlx::query!(
        r#"
        SELECT (missing_since IS NOT NULL) AS "is_missing!"
        FROM media_item
        WHERE relative_path = $1
        "#,
        relative_path
    )
    .fetch_optional(pool)
    .await?;

    if let Some(item) = existing {
        if item.is_missing {
            info!("Re-discovered missing file on disk: {relative_path}, clearing missing status.");
            MediaItemStore::unmark_relative_paths_as_missing(pool, &[relative_path]).await?;
        }
        return Ok(());
    }

    enqueue_full_ingest(pool, settings, &relative_path, user.id, None).await?;
    Ok(())
}
