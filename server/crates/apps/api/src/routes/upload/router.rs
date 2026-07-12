use crate::api_state::ApiContext;
use axum::Router;
use fileloft_axum::tus_router;
use fileloft_core::{Config, TusHandler};
use fileloft_store_fs::{FileLocker, FileStore};
use std::sync::Arc;

pub fn upload_protected_router(api_state: &ApiContext) -> Router<ApiContext> {
    let tus_dir = api_state.settings.ingest.app_data_root.join("tus_uploads");
    let locks_dir = tus_dir.join("locks");

    std::fs::create_dir_all(&tus_dir).ok();
    std::fs::create_dir_all(&locks_dir).ok();

    let store = FileStore::new(&tus_dir);
    let locker = FileLocker::new(&locks_dir);
    // todo: if uploads fail or are abandoned, files will pile up in the /tus_uploads folder
    // todo: what if two users upload the same named file? maybe there should be some per user folder logic
    let handler = Arc::new(TusHandler::new(store, Some(locker), Config::default()));

    let tus_nested = tus_router(handler).with_state(());

    Router::new().nest("/files", tus_nested)
}
