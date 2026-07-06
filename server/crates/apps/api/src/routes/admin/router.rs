use crate::admin::handlers::{
    delete_user_handler, get_disk_response, get_folder_media_sample, get_folder_unsupported,
    get_folders, get_users, make_folder, update_user_media_folder_handler,
};
use crate::api_state::ApiContext;
use axum::routing::{delete, put};
use axum::{
    Router,
    routing::{get, post},
};

pub fn admin_admin_routes() -> Router<ApiContext> {
    Router::new()
        .route("/admin/disk-info", get(get_disk_response))
        .route("/admin/media-sample", get(get_folder_media_sample))
        .route("/admin/unsupported-files", get(get_folder_unsupported))
        .route("/admin/folders", get(get_folders))
        .route("/admin/make-folder", post(make_folder))
        .route("/admin/users", get(get_users))
        .route(
            "/admin/users/{id}/media-folder",
            put(update_user_media_folder_handler),
        )
        .route("/admin/users/{id}", delete(delete_user_handler))
}
