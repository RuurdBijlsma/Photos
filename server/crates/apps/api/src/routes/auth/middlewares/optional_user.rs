use crate::api_state::ApiContext;
use crate::auth::middlewares::common::{decode_token, extract_access_token, extract_context};
use crate::auth::middlewares::user::ApiUser;
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use common_services::api::auth::error::AuthError;

#[derive(Clone, Debug)]
pub struct OptionalUser(pub Option<ApiUser>);

impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync,
    State<ApiContext>: FromRequestParts<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match extract_access_token(parts) {
            Ok(token) => {
                let context = extract_context(parts, state).await?;
                let claims = decode_token(&token, &context.settings.secrets.jwt)?;
                let user = ApiUser {
                    expiry: claims.exp,
                    id: claims.sub,
                    role: claims.role,
                };
                parts.extensions.insert(Self(Some(user.clone())));
                Ok(Self(Some(user)))
            }
            Err(AuthError::MissingToken) => {
                parts.extensions.insert(Self(None));
                Ok(Self(None))
            }
            Err(e) => Err(e),
        }
    }
}
