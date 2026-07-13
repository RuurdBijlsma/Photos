use crate::api_state::ApiContext;
use crate::upload::handlers::{get_upload_jwt, handle_upload_finished, pre_create_handler};
use app_state::constants::{TUS_LOCKS_FOLDER, TUS_UPLOADS_FOLDER};
use axum::Router;
use axum::routing::get;
use fileloft_axum::tus_router;
use fileloft_core::hooks::HookConfig;
use fileloft_core::{Config, Extensions, HookEvent, TusHandler};
use fileloft_store_fs::{FileLocker, FileStore};
use std::sync::Arc;
use std::time::Duration;

fn get_tus_handler(context: &ApiContext) -> TusHandler<FileStore, FileLocker> {
    let app_data_root = &context.settings.ingest.app_data_root;
    let tus_dir = app_data_root.join(TUS_UPLOADS_FOLDER);
    let locks_dir = app_data_root.join(TUS_LOCKS_FOLDER);

    let store = FileStore::new(&tus_dir);
    let locker = FileLocker::new(&locks_dir);

    let extensions = Extensions {
        expiration: true,
        expiration_ttl: Some(Duration::from_hours(72)),
        ..Default::default()
    };

    // Subscribe to pre_create hook to handle auth
    let create_context = context.clone();
    let config = Config {
        extensions,
        hooks: HookConfig {
            channel_capacity: 2048,
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
            loop {
                match rx.recv().await {
                    Ok(HookEvent::UploadFinished { info }) => {
                        handle_upload_finished(&api_state_clone, &info).await;
                    }
                    Ok(_) => {} // Ignore other hook events if they occur
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                        "Upload event loop lagged. Skipped {skipped} events."
                    );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Upload hook channel closed. Exiting event loop task.");
                        break;
                    }
                }
            }
        });
    }

    handler
}

pub fn upload_protected_router() -> Router<ApiContext> {
    // todo, can i do multi file upload or something? So i can batch enqueue the jobs for it as well?
    Router::new().route("/upload/jwt", get(get_upload_jwt))
}

// Public because the auth is in the jwt token which is validated in `pre_create_handler`
pub fn upload_public_router(api_state: &ApiContext) -> Router<ApiContext> {
    let tus = Arc::new(get_tus_handler(api_state));
    Router::new().nest("/files", tus_router(tus).with_state(()))
}
