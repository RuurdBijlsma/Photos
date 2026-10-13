use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use common_services::api::app_error::AppError;
use common_services::api::google_photos::interfaces::{CommitImportRequest, ExchangeCodeRequest, GooglePhotosConfig, GooglePhotosConfigStatus, GooglePhotosUserStatus, PollResult, SaveGoogleConfigRequest, StartPickerResponse};
use common_services::api::google_photos::service::{commit_import, get_google_config_status, get_google_user_status, poll_import_session, save_google_config, start_picker};
use common_services::database::album::album::Album;


// ─── Admin handlers ───────────────────────────────────────────────────────────

/// Returns whether Google Photos integration is configured (admin only).
pub async fn get_config_status_handler(
    State(context): State<ApiContext>,
) -> Result<Json<GooglePhotosConfigStatus>, AppError> {
    let status = get_google_config_status(&context.pool).await?;
    Ok(Json(status))
}

/// Save / overwrite the Google Photos OAuth credentials (admin only).
pub async fn save_config_handler(
    State(context): State<ApiContext>,
    Json(payload): Json<SaveGoogleConfigRequest>,
) -> Result<StatusCode, AppError> {
    let config = GooglePhotosConfig {
        client_id: payload.client_id,
        client_secret: payload.client_secret,
    };
    save_google_config(&context.pool, config).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── User handlers ────────────────────────────────────────────────────────────

/// Exchange an OAuth authorization code for an access token, create a Picker
/// session, and return the picker URI to the frontend.
pub async fn start_picker_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<ExchangeCodeRequest>,
) -> Result<Json<StartPickerResponse>, AppError> {
    let response = start_picker(&context.pool, user.id, payload).await?;
    Ok(Json(response))
}

/// Poll an import session to check whether the user has finished selecting in
/// Google Photos. Returns matched/unmatched item lists when ready.
pub async fn poll_import_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(import_session_id): Path<String>,
) -> Result<Json<PollResult>, AppError> {
    let result = poll_import_session(&context.pool, &import_session_id, user.id).await?;
    Ok(Json(result))
}

/// Commit an import: create a local album and add matched items.
pub async fn commit_import_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<CommitImportRequest>,
) -> Result<Json<Album>, AppError> {
    let album = commit_import(&context.pool, user.id, payload).await?;
    Ok(Json(album))
}

/// Returns whether Google Photos is configured and the client ID for OAuth initiation (any logged-in user).
pub async fn get_user_status_handler(
    State(context): State<ApiContext>,
) -> Result<Json<GooglePhotosUserStatus>, AppError> {
    let status = get_google_user_status(&context.pool).await?;
    Ok(Json(status))
}
