use crate::api_state::ApiContext;
use crate::upload::handlers::{upload_poc_handler, upload_complete_handler};
use axum::{routing::{get, post}, Router};
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
    let handler = Arc::new(TusHandler::new(store, Some(locker), Config::default()));

    let tus_nested = tus_router(handler).with_state(());

    Router::new()
        .route("/upload/poc", get(upload_poc_handler))
        .route("/upload/complete", post(upload_complete_handler))
        .nest("/files", tus_nested) // <--- Match default fileloft mounting prefix
}