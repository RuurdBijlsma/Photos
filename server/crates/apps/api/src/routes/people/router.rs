use crate::api_state::ApiContext;
use crate::people::handlers::{get_person_media_item_id, get_person_thumbnail_redirect_handler};
use crate::routes::people::handlers::{
    get_person_photos_handler, list_people_handler, merge_person_handler, unmerge_person_handler,
    update_person_handler,
};
use axum::Router;
use axum::routing::{get, post};

pub fn people_public_router() -> Router<ApiContext> {
    Router::new().route(
        "/people/{id}/thumbnail",
        get(get_person_thumbnail_redirect_handler),
    )
}

pub fn people_protected_router() -> Router<ApiContext> {
    Router::new()
        .route("/people", get(list_people_handler))
        .route("/people/{id}/merge", post(merge_person_handler))
        .route("/people/{id}/unmerge", post(unmerge_person_handler))
        .route("/people/{id}/media-item-id", get(get_person_media_item_id))
        .route(
            "/people/{id}",
            get(get_person_photos_handler).patch(update_person_handler),
        )
}
