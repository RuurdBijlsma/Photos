use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use app_state::{AppSettings, constants};
use axum::extract::State;
use axum::{Extension, Json};
use chrono::{Duration, Utc};
use common_services::alert;
use common_services::api::app_error::AppError;
use common_services::api::auth::interfaces::AuthClaims;
use common_services::api::upload::interfaces::{TakeoutBatchPayload, TakeoutStartResponse};
use common_services::api::upload::service::{decode_upload_token, repair_tus_upload};
use common_services::database::jobs::JobType;
use common_services::job_queue::enqueue_job;
use fileloft_core::info::UploadInfoChanges;
use fileloft_core::{Metadata, TusError, UploadInfo};
use jsonwebtoken::{EncodingKey, Header, encode};
use std::collections::HashSet;
use tracing::{error, warn};

pub async fn get_upload_jwt(
    State(settings): State<AppSettings>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<String>, AppError> {
    let exp =
        (Utc::now() + Duration::minutes(constants().auth.access_token_expiry_minutes)).timestamp();
    let claims = AuthClaims {
        sub: user.id,
        role: user.role,
        exp,
        aud: "upload".to_string(),
    };
    // todo: its probably not safe to store the jwt secret in the config YAML
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(settings.secrets.jwt.as_ref()),
    )
    .map_err(Into::<AppError>::into)?;

    Ok(Json(access_token))
}

// Decodes the JWT, extracts user ID, and overwrites any user-submitted userId key.
#[allow(clippy::unused_async)]
pub async fn pre_create_handler(
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

pub async fn handle_upload_finished(context: &ApiContext, info: &UploadInfo) {
    let Some(user_id) = info
        .metadata
        .get("userId")
        .and_then(|opt| opt.as_ref())
        .and_then(|s| s.parse::<i32>().ok())
    else {
        alert!(
            "Couldn't process user_id in upload finished handler: {:?}, {:?}",
            info.metadata.get("userId"),
            &info
        );
        return;
    };

    // Takeout ZIPs are imported explicitly once the browser's upload queue is done.
    // A finished ZIP on its own says nothing about whether the whole export has arrived.
    let is_takeout = info
        .metadata
        .get("isTakeout")
        .and_then(|opt| opt.as_ref())
        .is_some_and(|v| v == "true");
    if is_takeout {
        return;
    }

    let enqueue_res = enqueue_job(
        &context.pool,
        &context.settings.ingest,
        JobType::HandleWebUpload,
    )
    .user_id(user_id)
    .payload(&info)
    .call()
    .await;

    if let Err(e) = enqueue_res {
        error!("Failed to enqueue HandleWebUpload job: {}", e);
    }
}

/// Completed Takeout ZIP uploads of `user_id` that no queued/running import already owns.
async fn pending_takeout_upload_ids(
    context: &ApiContext,
    user_id: i32,
) -> Result<Vec<String>, AppError> {
    let claimed_payloads: Vec<serde_json::Value> = sqlx::query_scalar(
        r"
        SELECT payload FROM jobs
        WHERE user_id = $1
          AND job_type = 'ingest_takeout_batch'::job_type
          AND status IN ('queued'::job_status, 'running'::job_status)
          AND payload IS NOT NULL
        ",
    )
    .bind(user_id)
    .fetch_all(&context.pool)
    .await?;
    let claimed: HashSet<String> = claimed_payloads
        .into_iter()
        .filter_map(|payload| serde_json::from_value::<TakeoutBatchPayload>(payload).ok())
        .flat_map(|payload| payload.tus_upload_ids)
        .collect();

    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(constants::TUS_UPLOADS_FOLDER);
    let mut upload_ids = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(&tus_dir).await else {
        return Ok(upload_ids);
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("info") {
            continue;
        }
        let Ok(info_json) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let Ok(info) = serde_json::from_str::<UploadInfo>(&info_json) else {
            continue;
        };
        let is_takeout = info
            .metadata
            .get("isTakeout")
            .and_then(|opt| opt.as_ref())
            .is_some_and(|v| v == "true");
        let owner = info
            .metadata
            .get("userId")
            .and_then(|opt| opt.as_ref())
            .and_then(|s| s.parse::<i32>().ok());
        if !is_takeout || owner != Some(user_id) || info.size != Some(info.offset) {
            continue;
        }
        let id = info.id.to_string();
        if claimed.contains(&id) {
            continue;
        }
        match repair_tus_upload(&tus_dir, &id).await {
            Ok(_) => upload_ids.push(id),
            Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                return Err(AppError::Conflict(format!(
                    "Takeout upload {id} is incomplete: {e}. Wait for it to finish or reselect the ZIPs."
                )));
            }
            Err(e) => return Err(AppError::Internal(e.into())),
        }
    }
    upload_ids.sort_unstable();
    upload_ids.dedup();
    Ok(upload_ids)
}

/// Starts the import for every finished Takeout ZIP of the current user.
pub async fn start_takeout_import(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<TakeoutStartResponse>, AppError> {
    let tus_upload_ids = pending_takeout_upload_ids(&context, user.id).await?;
    if tus_upload_ids.is_empty() {
        return Ok(Json(TakeoutStartResponse {
            started: false,
            zip_count: 0,
        }));
    }

    let zip_count = tus_upload_ids.len();
    let payload = TakeoutBatchPayload { tus_upload_ids };
    enqueue_job(
        &context.pool,
        &context.settings.ingest,
        JobType::IngestTakeoutBatch,
    )
    .user_id(user.id)
    .payload(&payload)
    .call()
    .await?;

    Ok(Json(TakeoutStartResponse {
        started: true,
        zip_count,
    }))
}
