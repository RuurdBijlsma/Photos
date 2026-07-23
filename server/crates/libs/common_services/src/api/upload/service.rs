use crate::api::app_error::AppError;
use crate::api::auth::interfaces::AuthClaims;
use jsonwebtoken::{DecodingKey, Validation, decode};

pub fn decode_upload_token(token: &str, jwt_secret: &str) -> Result<AuthClaims, AppError> {
    let mut validation = Validation::default();
    validation.set_audience(&["upload"]);

    decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized("Bad token".to_string()))
}
