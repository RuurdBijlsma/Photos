use crate::api_state::ApiContext;
use crate::mobile::handlers::get_timeline_sync_handler;
use axum::{Router, routing::get};

pub fn mobile_protected_router() -> Router<ApiContext> {
    Router::new().route("/mobile/timeline/sync", get(get_timeline_sync_handler))
}
