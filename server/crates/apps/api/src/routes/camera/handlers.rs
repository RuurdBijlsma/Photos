use crate::auth::middlewares::user::ApiUser;
use axum::Extension;
use axum::extract::{Path, State};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::camera::service::{get_all_cameras, get_camera_photos};
use common_types::pb::api::{FullCameraPhotosResponse, ListCameraResponse};
use sqlx::PgPool;
use tracing::instrument;

#[instrument(skip(pool, user), err(Debug))]
pub async fn list_cameras_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Protobuf<ListCameraResponse>, AppError> {
    let result = get_all_cameras(&pool, user.id).await?;
    Ok(Protobuf(result))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_camera_photos_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Path((camera_make, camera_model)): Path<(String, String)>,
) -> Result<Protobuf<FullCameraPhotosResponse>, AppError> {
    let result = get_camera_photos(&pool, &camera_make, &camera_model, user.id).await?;
    Ok(Protobuf(result))
}
