use crate::api_state::ApiContext;
use crate::s2s::handlers::{download_file_handler, invite_summary_handler};
use axum::{Router, routing::get};

pub fn s2s_public_router() -> Router<ApiContext> {
    Router::new()
        .route("/s2s/album/invite-summary", get(invite_summary_handler))
        .route("/s2s/album/files", get(download_file_handler))
}
