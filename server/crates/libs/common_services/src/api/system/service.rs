use crate::api::app_error::AppError;
use crate::api::system::interfaces::{DiskStats, SystemStats};
use crate::api::system::storage_helpers::{
    ARE_SAME_DRIVE, are_on_same_drive, get_single_disk_info,
};
use app_state::{IngestSettings, constants};
use sqlx::PgPool;
use tokio::task;

pub async fn get_system_stats(
    pool: &PgPool,
    settings: &IngestSettings,
    user_id: i32,
) -> Result<SystemStats, AppError> {
    let db_task = sqlx::query!(
        r#"
        SELECT
            EXISTS(SELECT 1 FROM person WHERE user_id = $1) AS "has_people!",
            EXISTS(SELECT 1 FROM photo_cluster WHERE user_id = $1) AS "has_photo_clusters!"
        "#,
        user_id
    )
    .fetch_one(pool);

    let app_data_folder = settings.app_data_root.clone();
    let media_folder = settings.media_root.clone();

    let fs_task = task::spawn_blocking(move || {
        let are_same_drive =
            *ARE_SAME_DRIVE.get_or_init(|| are_on_same_drive(&app_data_folder, &media_folder));

        let app_data_drive = get_single_disk_info(&app_data_folder)?;
        let media_drive = if are_same_drive {
            app_data_drive.clone()
        } else {
            get_single_disk_info(&media_folder)?
        };

        Ok::<_, AppError>(DiskStats {
            app_data_drive,
            media_drive,
            are_same_drive,
        })
    });

    let ingesting_task = is_user_ingesting(pool, user_id);

    let (db_res, ingest_res, fs_res) = tokio::try_join!(
        async { db_task.await.map_err(AppError::from) },
        ingesting_task,
        async { fs_task.await.map_err(AppError::from)? }
    )?;

    Ok(SystemStats {
        has_clustered_people: db_res.has_people,
        has_clustered_photos: db_res.has_photo_clusters,
        allow_file_deletion: constants().allow_file_deletion,
        allow_file_modifications: constants().allow_file_modifications,
        disk: fs_res,
        is_ingesting: ingest_res,
    })
}

/// Checks whether the user has active ingest tasks scheduled or actively running.
/// -- removed `ingest_llm` from this list
pub async fn is_user_ingesting(pool: &PgPool, user_id: i32) -> Result<bool, AppError> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM jobs
            WHERE user_id = $1
              AND status IN ('queued'::job_status, 'running'::job_status)
              AND job_type IN ('ingest_metadata', 'ingest_thumbnails', 'ingest_analysis')
        ) as "exists!"
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(exists)
}
