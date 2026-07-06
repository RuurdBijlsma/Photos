use crate::api_state::ApiContext;
use crate::search::handlers::{
    get_random_search_suggestion_handler, get_search_by_image_results, get_search_by_image_uuid,
    get_search_filter_ranges, get_search_results, get_search_suggestions_handler,
};
use axum::{
    Router,
    routing::{get, post},
};

pub fn search_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/search", get(get_search_results))
        .route("/search/image", post(get_search_by_image_results))
        .route("/search/image/uuid/{uuid}", get(get_search_by_image_uuid))
        .route("/search/params", get(get_search_filter_ranges))
        .route("/search/suggestions", get(get_search_suggestions_handler))
        .route(
            "/search/suggestions/random",
            get(get_random_search_suggestion_handler),
        )
}
