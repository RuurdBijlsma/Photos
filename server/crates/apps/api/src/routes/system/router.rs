use crate::api_state::ApiContext;
use crate::system::handlers::{get_public_system_stats_handler, get_system_stats_handler};
use axum::{Router, routing::get};

pub fn system_protected_router() -> Router<ApiContext> {
    Router::new().route("/system/stats", get(get_system_stats_handler))
}

pub fn system_public_router()->Router<ApiContext>{
    Router::new().route("/system/public/stats", get(get_public_system_stats_handler))
}
