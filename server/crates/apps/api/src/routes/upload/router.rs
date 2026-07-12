use crate::api_state::ApiContext;
use axum::Router;
use common_services::database::jobs::JobType;
use common_services::job_queue::enqueue_job;
use fileloft_axum::tus_router;
use fileloft_core::hooks::HookConfig;
use fileloft_core::{Config, TusError, TusHandler, UploadInfo};
use fileloft_store_fs::{FileLocker, FileStore};
use std::sync::Arc;
use tracing::warn;

// todo: clean up post find put it in handlers or service.rs
async fn post_fin(api_state: ApiContext, upload_info: UploadInfo) -> Result<(), TusError> {
    dbg!("Upload finished", &upload_info);

    // todo: get user-id from jwt from metadata header
    let user_id = 1;

    enqueue_job(
        &api_state.pool,
        &api_state.settings.ingest,
        JobType::HandleWebUpload,
    )
    .user_id(user_id)
    .payload(&upload_info)
    .call()
    .await
    .map_err(|e| {
        warn!("Enqueue job HandleWebUpload ERROR: {e}");
        TusError::Internal("Couldn't enqueue HandleWebUpload job".to_owned())
    })?;

    Ok(())
}

pub fn upload_protected_router(api_state: &ApiContext) -> Router<ApiContext> {
    let tus_dir = api_state.settings.ingest.app_data_root.join("tus_uploads");
    let locks_dir = tus_dir.join("locks");

    std::fs::create_dir_all(&tus_dir).ok();
    std::fs::create_dir_all(&locks_dir).ok();

    let store = FileStore::new(&tus_dir);
    let locker = FileLocker::new(&locks_dir);
    // todo: if uploads fail or are abandoned, files will pile up in the /tus_uploads folder

    let api_state_cloned = api_state.clone();

    // todo: if uploads fail or are abandoned, files will pile up in the /tus_uploads folder
    let config = Config {
        hooks: HookConfig {
            pre_finish: Some(Arc::new(move |info| {
                let cloned_state = api_state_cloned.clone();
                Box::pin(post_fin(cloned_state, info))
            })),
            ..Default::default()
        },
        ..Default::default()
    };
    let handler = Arc::new(TusHandler::new(store, Some(locker), config));

    let tus_nested = tus_router(handler).with_state(());

    // todo: add /upload/jwt endpoint to get a jwt with your user id so the uploaded file knows who it belongs to
    Router::new().nest("/files", tus_nested)
}
