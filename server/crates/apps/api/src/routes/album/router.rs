use crate::album::handlers::{
    delete_album_handler, get_album_media_handler, get_album_media_item_handler,
    get_sorted_album_items_handler, list_backups_handler, restore_backup_handler,
};
use crate::api_state::ApiContext;
use crate::routes::album::handlers::{
    accept_invite_handler, add_collaborator_handler, add_media_to_album_handler,
    check_invite_handler, create_album_handler, generate_invite_handler, get_user_albums_handler,
    remove_collaborator_handler, remove_media_from_album_handler, reorder_media_handler,
    update_album_handler,
};
use axum::routing::put;
use axum::{
    Router,
    routing::{delete, get, post},
};

pub fn album_auth_optional_router() -> Router<ApiContext> {
    Router::new()
        .route("/album/{album_id}", get(get_album_media_handler))
        .route(
            "/album/{album_id}/item/{media_item_id}",
            get(get_album_media_item_handler),
        )
}

pub fn album_protected_router() -> Router<ApiContext> {
    Router::new()
        .route(
            "/album",
            post(create_album_handler).get(get_user_albums_handler),
        )
        .route(
            "/album/{album_id}",
            put(update_album_handler).delete(delete_album_handler),
        )
        .route(
            "/album/{album_id}/media/sorted",
            get(get_sorted_album_items_handler),
        )
        .route("/album/{album_id}/media", post(add_media_to_album_handler))
        .route(
            "/album/{album_id}/media/reorder",
            put(reorder_media_handler),
        )
        .route(
            "/album/{album_id}/media/{media_item_ids}",
            delete(remove_media_from_album_handler),
        )
        .route(
            "/album/{album_id}/collaborators",
            post(add_collaborator_handler),
        )
        .route(
            "/album/{album_id}/collaborators/{collaborator_id}",
            delete(remove_collaborator_handler),
        )
        .route("/album/{album_id}/invite", get(generate_invite_handler))
        .route("/album/invite/check", post(check_invite_handler))
        .route("/album/invite/accept", post(accept_invite_handler))
        .route("/album/backup/list", get(list_backups_handler))
        .route("/album/restore/{backup_file}", post(restore_backup_handler))
}
