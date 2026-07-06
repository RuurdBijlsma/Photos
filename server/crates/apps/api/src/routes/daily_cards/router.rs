use crate::api_state::ApiContext;
use crate::daily_cards::handlers::{get_daily_cards_handler, validate_media_handler};
use axum::{
    Router,
    routing::{get, post},
};

pub fn daily_cards_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/daily-cards", get(get_daily_cards_handler))
        .route("/daily-cards/validate-media", post(validate_media_handler))
}
