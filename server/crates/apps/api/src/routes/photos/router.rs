use crate::api_state::ApiContext;

use crate::photos::handlers::{
    download_full_file_by_id, download_full_file_by_rel_path, get_full_item_handler,
    get_geo_photos_handler, get_photo_thumbnail, stream_video_handler, update_media_item_handler,
};
use axum::{Router, routing::get};

pub fn photos_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/photos/geo", get(get_geo_photos_handler))
        .route(
            "/photos/{media_item_id}/item",
            get(get_full_item_handler).put(update_media_item_handler),
        )
        .route(
            "/photos/{media_item_id}/download",
            get(download_full_file_by_id),
        )
        .route("/photos/download", get(download_full_file_by_rel_path))
}

pub fn photos_public_router() -> Router<ApiContext> {
    Router::new()
        .route(
            "/photos/{media_item_id}/thumbnail",
            get(get_photo_thumbnail),
        )
        .route("/photos/{media_item_id}/video", get(stream_video_handler))
}
