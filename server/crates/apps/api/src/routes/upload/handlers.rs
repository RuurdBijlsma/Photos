use crate::auth::middlewares::user::ApiUser;
use app_state::{AppSettings, constants};
use axum::extract::State;
use axum::{Extension, Json};
use chrono::{Duration, Utc};
use common_services::api::app_error::AppError;
use common_services::api::auth::interfaces::AuthClaims;
use jsonwebtoken::{EncodingKey, Header, encode};

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
