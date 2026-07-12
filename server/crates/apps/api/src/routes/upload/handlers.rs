use axum::extract::{Query, State};
use axum::{Extension, Json};

use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use common_services::api::app_error::AppError;
use common_services::api::theme::interfaces::{RandomPhotoParams};

pub async fn upload_poc_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<RandomPhotoParams>,
) -> Result<Json<()>, AppError> {
    // todo implement
    Ok(Json(()))
}
