use crate::api_state::ApiContext;
use axum::Extension;
use axum::extract::{Path, State};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::camera::service::{get_all_cameras, get_camera_photos};
use common_services::database::app_user::User;
use common_types::pb::api::{FullCameraPhotosResponse, ListCameraResponse};
use tracing::instrument;

#[instrument(skip(context, user), err(Debug))]
pub async fn list_cameras_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Protobuf<ListCameraResponse>, AppError> {
    let result = get_all_cameras(&context.pool, user.id).await?;
    Ok(Protobuf(result))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_camera_photos_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path((camera_make, camera_model)): Path<(String, String)>,
) -> Result<Protobuf<FullCameraPhotosResponse>, AppError> {
    let result = get_camera_photos(&context.pool, &camera_make, &camera_model, user.id).await?;
    Ok(Protobuf(result))
}
