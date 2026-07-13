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

/// Parses a specific cookie's value from the HTTP request headers.
pub fn extract_cookie_value(parts: &Parts, cookie_name: &str) -> Option<String> {
    parts
        .headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find(|cookie| cookie.starts_with(&format!("{cookie_name}=")))
        .and_then(|cookie| {
            let mut key_val = cookie.splitn(2, '=');
            key_val.next(); // Skip cookie name
            key_val.next().map(ToString::to_string)
        })
}

pub fn extract_access_token(parts: &Parts) -> Result<String, AuthError> {
    extract_cookie_value(parts, "access_token").ok_or(AuthError::MissingToken)
}

pub fn extract_refresh_token(parts: &Parts) -> Result<String, AuthError> {
    extract_cookie_value(parts, "refresh_token").ok_or(AuthError::MissingToken)
}

pub fn extract_token(parts: &Parts) -> Result<String, AuthError> {
    extract_access_token(parts)
}

pub fn decode_token(token: &str, jwt_secret: &str) -> Result<AuthClaims, AuthError> {
    let mut validation = Validation::default();
    validation.set_audience(&["api"]);

    decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| AuthError::InvalidToken)
}
