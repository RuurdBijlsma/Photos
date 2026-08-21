use crate::api_state::ApiContext;

use crate::photos::handlers::{download_full_file_by_id, download_full_file_by_rel_path, get_full_item_handler, get_geo_photos_handler, get_pano_config, get_photo_thumbnail, reprocess_media_item_handler, stream_video_handler, update_media_item_handler};
use crate::photos::zip_handler::download_zip_stream_handler;
use axum::{Router, routing::get};
use axum::routing::post;

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
        .route("/photos/{media_item_id}/reprocess", post(reprocess_media_item_handler))
        .route("/photos/download", get(download_full_file_by_rel_path))
        .route("/photos/download/zip", get(download_zip_stream_handler))
        .route("/photos/{media_item_id}/video", get(stream_video_handler))
}

pub fn photos_public_router() -> Router<ApiContext> {
    Router::new()
        .route(
            "/photos/{media_item_id}/thumbnail",
            get(get_photo_thumbnail),
        )
        .route("/photos/{media_item_id}/pano-config", get(get_pano_config))
}
