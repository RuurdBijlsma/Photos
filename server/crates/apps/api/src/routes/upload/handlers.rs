use axum::extract::State;
use axum::{Extension, Json};

use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use common_services::api::app_error::AppError;
use common_services::api::upload::interfaces::UploadCompletePayload;
use common_services::api::upload::service::handle_upload_complete;


pub async fn upload_complete_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<UploadCompletePayload>,
) -> Result<Json<()>, AppError> {
    handle_upload_complete(&context.pool, &context.settings.ingest, user.id, payload)
        .await
        .map_err(|e| AppError::NotFound(format!("Upload post-processing failed: {e}")))?;

    Ok(Json(()))
}