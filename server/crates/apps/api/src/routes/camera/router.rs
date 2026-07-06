use crate::api_state::ApiContext;
use crate::camera::handlers::{get_camera_photos_handler, list_cameras_handler};
use axum::Router;
use axum::routing::get;

pub fn camera_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/camera", get(list_cameras_handler))
        .route(
            "/camera/{camera_make}/{camera_model}",
            get(get_camera_photos_handler),
        )
}
