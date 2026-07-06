use crate::api_state::ApiContext;
use crate::system::handlers::get_system_stats_handler;
use axum::{Router, routing::get};

pub fn system_protected_router() -> Router<ApiContext> {
    Router::new().route("/system/stats", get(get_system_stats_handler))
}
