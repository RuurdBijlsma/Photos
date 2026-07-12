use crate::api_state::ApiContext;

use axum::{routing::get, Router};
use crate::upload::handlers::upload_poc_handler;

pub fn upload_protected_router() -> Router<ApiContext> {
    // todo implement
    Router::new()
        .route("/upload/poc", get(upload_poc_handler))
}
