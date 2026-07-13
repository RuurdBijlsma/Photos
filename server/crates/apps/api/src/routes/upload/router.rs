use crate::api_state::ApiContext;
use crate::upload::handlers::get_upload_jwt;
use axum::Router;
use axum::routing::get;
use common_services::api::upload::service::decode_upload_token;
use common_services::database::jobs::JobType;
use common_services::job_queue::enqueue_job;
use fileloft_axum::tus_router;
use fileloft_core::hooks::HookConfig;
use fileloft_core::info::UploadInfoChanges;
use fileloft_core::{Config, Metadata, TusError, TusHandler, UploadInfo};
use fileloft_store_fs::{FileLocker, FileStore};
use std::sync::Arc;
use tracing::warn;

// todo: clean up post find put it in handlers or service.rs
/// Decodes the JWT, extracts user ID, and overwrites any user-submitted userId key.
#[allow(clippy::unused_async)]
async fn pre_create(
    api_state: ApiContext,
    info: UploadInfo,
) -> Result<UploadInfoChanges, TusError> {
    let jwt_str = info
        .metadata
        .get("jwt")
        .and_then(|opt| opt.as_ref())
        .ok_or_else(|| TusError::InvalidMetadata("Missing 'jwt' in upload metadata".to_owned()))?;
    let claims = decode_upload_token(jwt_str, &api_state.settings.secrets.jwt)
        .map_err(|e| TusError::InvalidMetadata(format!("Invalid or expired upload token: {e}")))?;

    let mut modified_metadata = info.metadata.0.clone();
    modified_metadata.remove("jwt");
    modified_metadata.insert("userId".to_owned(), Some(claims.sub.to_string()));

    Ok(UploadInfoChanges {
        metadata: Some(Metadata(modified_metadata)),
        ..Default::default()
    })
}

async fn pre_finish(api_state: ApiContext, upload_info: UploadInfo) -> Result<(), TusError> {
    dbg!("Upload finished", &upload_info);

    let user_id = upload_info
        .metadata
        .get("userId")
        .and_then(|opt| opt.as_ref())
        .ok_or_else(|| TusError::Internal("Missing userId metadata".to_owned()))?
        .parse::<i32>()
        .map_err(|_| TusError::Internal("Malformed userId metadata".to_owned()))?;

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

    let create_context = api_state.clone();
    let finish_context = api_state.clone();

    let config = Config {
        hooks: HookConfig {
            pre_create: Some(Arc::new(move |info| {
                let cloned_state = create_context.clone();
                Box::pin(pre_create(cloned_state, info))
            })),
            pre_finish: Some(Arc::new(move |info| {
                let cloned_state = finish_context.clone();
                Box::pin(pre_finish(cloned_state, info))
            })),
            ..Default::default()
        },
        ..Default::default()
    };
    let handler = Arc::new(TusHandler::new(store, Some(locker), config));
    let tus_nested = tus_router(handler).with_state(());

    Router::new()
        .route("/upload/jwt", get(get_upload_jwt))
        .nest("/files", tus_nested)
}
