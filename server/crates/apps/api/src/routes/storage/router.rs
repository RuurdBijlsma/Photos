use crate::api_state::ApiContext;
use crate::storage::handlers::{
    prune_missing_handler, storage_blurry_handler, storage_missing_handler, storage_review_handler,
    storage_summary_handler,
};
use axum::Router;
use axum::routing::{get, post};

pub fn storage_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/storage/summary", get(storage_summary_handler))
        .route("/storage/review", get(storage_review_handler))
        .route("/storage/blurry", get(storage_blurry_handler))
        .route("/storage/missing", get(storage_missing_handler))
        .route("/storage/missing/prune", post(prune_missing_handler))
}
