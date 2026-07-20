use crate::api_state::ApiContext;
use axum::{
    Router,
    routing::get,
};
use crate::explore::handlers::{get_explore_table_handler, get_histograms_handler};

pub fn explore_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/explore/table", get(get_explore_table_handler))
        .route("/explore/histograms", get(get_histograms_handler))
}