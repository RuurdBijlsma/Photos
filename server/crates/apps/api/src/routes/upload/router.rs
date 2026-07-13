use crate::api_state::ApiContext;
use crate::upload::handlers::{get_upload_jwt, handle_upload_finished, pre_create_handler};
use app_state::constants::{TUS_LOCKS_FOLDER, TUS_UPLOADS_FOLDER};
use axum::Router;
use axum::routing::get;
use fileloft_axum::tus_router;
use fileloft_core::hooks::HookConfig;
use fileloft_core::{Config, HookEvent, TusHandler};
use fileloft_store_fs::{FileLocker, FileStore};
use std::sync::Arc;

fn get_tus_handler(context: &ApiContext) -> TusHandler<FileStore, FileLocker> {
    let app_data_root = &context.settings.ingest.app_data_root;
    let tus_dir = app_data_root.join(TUS_UPLOADS_FOLDER);
    let locks_dir = app_data_root.join(TUS_LOCKS_FOLDER);

    let store = FileStore::new(&tus_dir);
    let locker = FileLocker::new(&locks_dir);
    // todo: if uploads fail or are abandoned, files will pile up in the /tus/uploads folder

    // Subscribe to pre_create hook to handle auth
    let create_context = context.clone();
    let config = Config {
        hooks: HookConfig {
            channel_capacity: 100,
            pre_create: Some(Arc::new(move |info| {
                let cloned_state = create_context.clone();
                Box::pin(pre_create_handler(cloned_state, info))
            })),
            ..Default::default()
        },
        ..Default::default()
    };
    let handler = TusHandler::new(store, Some(locker), config);

    // Subscribe to upload finished event
    if let Some(mut rx) = handler.hook_receiver() {
        let api_state_clone = context.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                if let HookEvent::UploadFinished { info } = event {
                    handle_upload_finished(&api_state_clone, &info).await;
                }
            }
        });
    }

    handler
}

pub fn upload_protected_router(api_state: &ApiContext) -> Router<ApiContext> {
    let tus = Arc::new(get_tus_handler(api_state));

    Router::new()
        .route("/upload/jwt", get(get_upload_jwt))
        .nest("/files", tus_router(tus).with_state(()))
}
