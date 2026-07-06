use crate::api_state::ApiContext;
use axum::extract::{FromRequestParts, State};
use color_eyre::eyre::eyre;
use common_services::api::auth::error::AuthError;
use common_services::api::auth::interfaces::AuthClaims;
use http::header;
use http::request::Parts;
use jsonwebtoken::{DecodingKey, Validation, decode};

pub async fn extract_context<S>(parts: &mut Parts, state: &S) -> Result<ApiContext, AuthError>
where
    S: Send + Sync,
    State<ApiContext>: FromRequestParts<S>,
{
    match State::<ApiContext>::from_request_parts(parts, state).await {
        Ok(State(context)) => Ok(context),
        Err(_e) => Err(AuthError::Internal(eyre!(
            "Server state is not configured correctly."
        ))),
    }
}

/// Get auth token from Authorization Header.
pub fn extract_token(parts: &Parts) -> Result<String, AuthError> {
    let auth_header = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::MissingToken)?;

    auth_header
        .strip_prefix("Bearer ")
        .map(ToOwned::to_owned)
        .ok_or(AuthError::InvalidToken)
}

pub fn decode_token(token: &str, jwt_secret: &str) -> Result<AuthClaims, AuthError> {
    decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AuthError::InvalidToken)
}
