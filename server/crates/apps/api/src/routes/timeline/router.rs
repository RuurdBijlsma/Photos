use crate::api_state::ApiContext;
use crate::timeline::handlers::{
    get_photos_by_month_handler, get_timeline_ids_handler, get_timeline_ratios_handler,
};
use axum::{Router, routing::get};

pub fn timeline_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/timeline/ratios", get(get_timeline_ratios_handler))
        .route("/timeline/ids", get(get_timeline_ids_handler))
        .route("/timeline/by-month", get(get_photos_by_month_handler))
}