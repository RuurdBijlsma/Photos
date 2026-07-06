use crate::api_state::ApiContext;

use crate::theme::handlers::{get_color_theme_handler, get_random_photo_theme};
use axum::{Router, routing::get};

pub fn theme_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/theme/random-photo", get(get_random_photo_theme))
        .route("/theme/by-color", get(get_color_theme_handler))
}
