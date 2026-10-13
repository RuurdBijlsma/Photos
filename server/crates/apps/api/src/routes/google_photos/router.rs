use crate::api_state::ApiContext;
use crate::routes::google_photos::handlers::{
    commit_import_handler, get_config_status_handler, poll_import_handler, save_config_handler,
    start_picker_handler,
};
use axum::routing::{get, post};
use axum::Router;
use crate::google_photos::handlers::get_user_status_handler;

/// Routes that require admin role.
pub fn google_photos_admin_router() -> Router<ApiContext> {
    Router::new()
        .route(
            "/admin/google-photos/config",
            get(get_config_status_handler).post(save_config_handler),
        )
}

/// Routes that require authentication (any user).
pub fn google_photos_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/google-photos/status", get(get_user_status_handler))
        .route("/google-photos/start", post(start_picker_handler))
        .route(
            "/google-photos/poll/{import_session_id}",
            get(poll_import_handler),
        )
        .route("/google-photos/commit", post(commit_import_handler))
}