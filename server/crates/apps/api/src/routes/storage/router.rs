use crate::api_state::ApiContext;
use crate::storage::handlers::{
    storage_blurry_handler, storage_review_handler, storage_summary_handler,
};
use axum::{Router, routing::get};

pub fn storage_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/storage/summary", get(storage_summary_handler))
        .route("/storage/review", get(storage_review_handler))
        .route("/storage/blurry", get(storage_blurry_handler))
}
