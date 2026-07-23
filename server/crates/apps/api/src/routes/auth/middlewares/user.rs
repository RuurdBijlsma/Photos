use crate::api_state::ApiContext;
use crate::auth::middlewares::common::{decode_token, extract_access_token, extract_context};
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use common_services::api::auth::error::AuthError;
use common_services::database::app_user::UserRole;

#[derive(Clone, Debug)]
pub struct ApiUser {
    pub id: i32,
    pub expiry: i64,
    pub role: UserRole,
}

impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
    State<ApiContext>: FromRequestParts<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_access_token(parts)?;
        let context = extract_context(parts, state).await?;
        let claims = decode_token(&token, &context.settings.secrets.jwt)?;
        let user = Self {
            expiry: claims.exp,
            id: claims.sub,
            role: claims.role,
        };
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}
