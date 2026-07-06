use crate::api_state::ApiContext;
use crate::auth::middlewares::common::{decode_token, extract_context, extract_token};
use axum::{
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use common_services::api::auth::error::AuthError;
use common_services::database::app_user::User;
use common_services::database::user_store::UserStore;

#[derive(Clone, Debug)]
pub struct ApiUser(pub User);

impl<S> FromRequestParts<S> for ApiUser
where
    S: Send + Sync,
    State<ApiContext>: FromRequestParts<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts)?;
        let context = extract_context(parts, state).await?;
        let claims = decode_token(&token, &context.settings.secrets.jwt)?;
        let user = UserStore::find_by_id(&context.pool, claims.sub)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        parts.extensions.insert(user.clone());
        Ok(Self(user))
    }
}
