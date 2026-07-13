use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use app_state::{AppSettings, constants};
use axum::extract::State;
use axum::{Extension, Json};
use chrono::{Duration, Utc};
use common_services::alert;
use common_services::api::app_error::AppError;
use common_services::api::auth::interfaces::AuthClaims;
use common_services::api::upload::service::decode_upload_token;
use common_services::database::jobs::JobType;
use common_services::job_queue::enqueue_job;
use fileloft_core::info::UploadInfoChanges;
use fileloft_core::{Metadata, TusError, UploadInfo};
use jsonwebtoken::{EncodingKey, Header, encode};
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
