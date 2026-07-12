use crate::auth::middlewares::user::ApiUser;
use axum::body::Body;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;
use common_services::api::auth::error::AuthError;
use common_services::database::app_user::UserRole;
use http::Request;

pub async fn require_role(
    State(required_role): State<UserRole>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = req
        .extensions()
        .get::<ApiUser>()
        .ok_or(AuthError::UserNotFound)?;

    if user.role != required_role {
        return Err(AuthError::PermissionDenied {
            user_id: user.id,
            path: req.uri().to_string(),
        });
    }

    Ok(next.run(req).await)
}
