use crate::context::WorkerContext;
use crate::handlers::JobResult;
use app_state::constants::{TUS_LOCKS_FOLDER, TUS_UPLOADS_FOLDER, USER_UPLOAD_FOLDER};
use chrono::Utc;
use color_eyre::Result;
use common_services::database::jobs::Job;
use common_services::database::user_store::UserStore;
use std::fs;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

/// Deletes expired refresh tokens from the database.
pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    // Delete 'done' jobs that are a week old
    sqlx::query!(
        "DELETE FROM jobs WHERE status = 'done' AND finished_at < $1",
        Utc::now() - Duration::from_hours(24 * 7)
    )
    .execute(&context.pool)
    .await?;
    // Delete 'failed' & 'cancelled' jobs that are 4 weeks old
    sqlx::query!(
        "DELETE FROM jobs WHERE status in ('failed', 'cancelled') AND finished_at < $1",
        Utc::now() - Duration::from_hours(24 * 7 * 4)
    )
    .execute(&context.pool)
    .await?;

    // Remove expired refresh tokens
    sqlx::query!(
        "DELETE FROM refresh_token WHERE expires_at < $1",
        Utc::now() - Duration::from_hours(1)
    )
    .execute(&context.pool)
    .await?;

    // Remove cached uploaded image embeddings once search sessions have expired.
    sqlx::query!(
        "DELETE FROM vision_embedding_cache WHERE created_at < $1",
        Utc::now() - Duration::from_hours(24)
    )
    .execute(&context.pool)
    .await?;

    // Sync media_count field on albums in case it drifted for some reason
    sqlx::query!(
        r"UPDATE album a
        SET media_count = (
            SELECT COUNT(*)
            FROM album_media_item ami
            JOIN media_item mi ON ami.media_item_id = mi.id
            WHERE ami.album_id = a.id AND mi.deleted = false
        )"
    )
    .execute(&context.pool)
    .await?;

    // Clean up old .uploading files in user upload folders and old files in TUS folders
    clean_old_upload_files(context).await?;

    Ok(JobResult::Done)
}

/// Cleans up expired `.uploading` files from user upload folders,
/// as well as expired files in `tus/uploads` and `tus/locks`.
async fn clean_old_upload_files(context: &WorkerContext) -> Result<()> {
    let cutoff = SystemTime::now() - Duration::from_hours(72);
    // todo fix ridiculous nesting and not using the result ?

    // Clean up old .uploading files in the user upload folder
    for user in UserStore::list_users(&context.pool).await? {
        let Some(user_media_folder) = &user.media_folder else {
            continue;
        };
        let user_upload_folder = context
            .settings
            .ingest
            .media_root
            .join(user_media_folder)
            .join(USER_UPLOAD_FOLDER);
        if !user_upload_folder.exists() {
            continue;
        }
        for entry in WalkDir::new(&user_upload_folder)
            .contents_first(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if entry.file_type().is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some("uploading")
                && let Ok(metadata) = fs::metadata(path)
                && let Ok(modified) = metadata.modified()
                && modified <= cutoff
            {
                tracing::info!("Removing expired uploading file: {:?}", path);
                fs::remove_file(path)?;
            }
        }
    }

    // Clean up old files in TUS folders (uploads & locks)
    let app_data_root = &context.settings.ingest.app_data_root;
    let tus_uploads = app_data_root.join(TUS_UPLOADS_FOLDER);
    let tus_locks = app_data_root.join(TUS_LOCKS_FOLDER);

    for dir in &[tus_uploads, tus_locks] {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir)
            .contents_first(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if entry.file_type().is_file()
                && let Ok(metadata) = fs::metadata(path)
                && let Ok(modified) = metadata.modified()
                && modified <= cutoff
            {
                tracing::info!("Removing expired TUS file: {:?}", path);
                fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}
