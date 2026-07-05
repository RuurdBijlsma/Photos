//! This module defines the HTTP handlers for the initial application onboarding process.

use crate::api_state::ApiContext;
use app_state::IngestSettings;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use common_services::api::admin::interfaces::{
    AdminUserInfo, DiskResponse, FolderParams, MakeFolderBody, MediaSampleResponse,
    UnsupportedFilesResponse, UpdateUserMediaFolderBody,
};
use common_services::api::admin::service::{
    admin_delete_user, admin_update_user_media_folder, create_folder, get_disks_info,
    get_folder_unsupported_files, get_media_sample, get_subfolders, is_valid_user_folder,
    list_admin_users,
};
use common_services::api::app_error::AppError;
use common_services::database::app_user::User;

/// Retrieves information about the configured media and thumbnail disks.
pub async fn get_disk_response(
    State(ingestion): State<IngestSettings>,
) -> Result<Json<DiskResponse>, AppError> {
    let disk_info = get_disks_info(&ingestion.media_root, &ingestion.app_data_root)?;
    Ok(Json(disk_info))
}

/// Retrieves a sample of media files from a specified folder.
pub async fn get_folder_media_sample(
    State(ingestion): State<IngestSettings>,
    Query(query): Query<FolderParams>,
) -> Result<Json<MediaSampleResponse>, AppError> {
    let user_path = is_valid_user_folder(&ingestion.media_root, &query.folder).await?;
    let response = get_media_sample(&ingestion, &user_path)?;
    Ok(Json(response))
}

/// Scans a folder and returns a list of unsupported file types.
pub async fn get_folder_unsupported(
    State(ingestion): State<IngestSettings>,
    Query(query): Query<FolderParams>,
) -> Result<Json<UnsupportedFilesResponse>, AppError> {
    let user_path = is_valid_user_folder(&ingestion.media_root, &query.folder).await?;
    let response = get_folder_unsupported_files(&ingestion, &user_path)?;
    Ok(Json(response))
}

/// Lists the subfolders within a given directory.
pub async fn get_folders(
    State(ingestion): State<IngestSettings>,
    Query(query): Query<FolderParams>,
) -> Result<Json<Vec<String>>, AppError> {
    let folders = get_subfolders(&ingestion, &query.folder).await?;
    Ok(Json(folders))
}

/// Creates a new folder within a specified base directory.
pub async fn make_folder(
    State(ingestion): State<IngestSettings>,
    Json(params): Json<MakeFolderBody>,
) -> Result<StatusCode, AppError> {
    create_folder(&ingestion.media_root, &params.base_folder, &params.new_name).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Retrieves a list of all users along with their statistics.
pub async fn get_users(
    State(context): State<ApiContext>,
) -> Result<Json<Vec<AdminUserInfo>>, AppError> {
    let users = list_admin_users(&context.pool, &context.settings).await?;
    Ok(Json(users))
}

/// Updates the media folder for a specific user and schedules processing.
pub async fn update_user_media_folder_handler(
    State(context): State<ApiContext>,
    Path(target_user_id): Path<i32>,
    Json(payload): Json<UpdateUserMediaFolderBody>,
) -> Result<StatusCode, AppError> {
    admin_update_user_media_folder(
        &context.pool,
        &context.settings.ingest,
        target_user_id,
        &payload.user_folder,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Deletes a user account.
pub async fn delete_user_handler(
    State(context): State<ApiContext>,
    Extension(current_user): Extension<User>,
    Path(target_user_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    admin_delete_user(&context.pool, target_user_id, current_user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
