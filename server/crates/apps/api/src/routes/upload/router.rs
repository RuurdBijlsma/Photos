use crate::api_state::ApiContext;
use crate::upload::handlers::{upload_complete_handler};
use axum::{routing::{post}, Router};
use std::sync::Arc;
use fileloft_core::{Config, TusHandler};
use fileloft_store_fs::{FileLocker, FileStore};
use fileloft_axum::tus_router;

pub fn upload_protected_router(api_state: &ApiContext) -> Router<ApiContext> {
    let tus_dir = api_state.settings.ingest.app_data_root.join("tus_uploads");
    let locks_dir = tus_dir.join("locks");

    std::fs::create_dir_all(&tus_dir).ok();
    std::fs::create_dir_all(&locks_dir).ok();

    let store = FileStore::new(&tus_dir);
    let locker = FileLocker::new(&locks_dir);
    // todo: if uploads fail or are abandoned, files will pile up in the /tus_uploads folder
    let handler = Arc::new(TusHandler::new(store, Some(locker), Config::default()));

    let tus_nested = tus_router(handler).with_state(());

    Router::new()
        // todo: instead of /upload/complete, make /upload/ticket
        // todo: this can create a JWT token for the client, which they can pass to Upload-Metadata header (see common_services/.../auth/interfaces.rs -> AuthClaims struct)
        // todo: so backend can use Tus hooks to determine the user id for the upload, and not require the user to call /upload/complete when it's done
        .route("/upload/complete", post(upload_complete_handler))
        // todo: research if it's needed to explicitly disable the body size limit
        .nest("/files", tus_nested)
}