use crate::api_state::ApiContext;
use crate::explore::handlers::{
    get_explore_table_handler, get_histograms_handler, get_location_handler,
    get_visited_places_handler,
};
use axum::{Router, routing::get};

pub fn explore_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/explore/table", get(get_explore_table_handler))
        .route("/explore/histograms", get(get_histograms_handler))
        .route("/explore/locations", get(get_visited_places_handler))
        .route(
            "/explore/locations/{location_key}",
            get(get_location_handler),
        )
}
