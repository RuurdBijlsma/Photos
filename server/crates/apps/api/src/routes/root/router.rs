use crate::api_state::ApiContext;
use crate::root::handlers::{health_check, root};
use axum::{Router, routing::get};

pub fn root_public_router() -> Router<ApiContext> {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
}
