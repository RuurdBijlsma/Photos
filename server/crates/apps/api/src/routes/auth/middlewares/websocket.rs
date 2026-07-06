use crate::api_state::ApiContext;
use crate::auth::middlewares::common::{decode_token, extract_context, extract_token};
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use common_services::api::auth::error::AuthError;
use common_services::database::{app_user::User, user_store::UserStore};

#[derive(Clone, Debug)]
pub struct WsUser(pub User);

fn extract_websocket_token(parts: &Parts) -> Result<String, AuthError> {
    if let Ok(token) = extract_token(parts) {
        return Ok(token);
    }

    let header = parts
        .headers
        .get("Sec-WebSocket-Protocol")
        .ok_or(AuthError::MissingToken)?
        .to_str()
        .map_err(|_| AuthError::InvalidToken)?;

    let parts: Vec<_> = header.split(',').map(str::trim).collect();
    let idx = parts.iter().position(|&p| p == "access_token");

    match idx {
        Some(i) if i + 1 < parts.len() => Ok(parts[i + 1].to_string()),
        _ => Err(AuthError::MissingToken),
    }
}

impl<S> FromRequestParts<S> for WsUser
where
    S: Send + Sync,
    State<ApiContext>: FromRequestParts<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_websocket_token(parts)?;

        let context = extract_context(parts, state).await?;
        let claims = decode_token(&token, &context.settings.secrets.jwt)?;

        match UserStore::find_by_id(&context.pool, claims.sub).await {
            Ok(Some(user)) => {
                parts.extensions.insert(user.clone());
                Ok(Self(user))
            }
            Ok(None) => Err(AuthError::UserNotFound),
            Err(e) => Err(AuthError::Internal(e.into())),
        }
    }
}
