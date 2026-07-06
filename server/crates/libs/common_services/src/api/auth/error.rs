use crate::api::app_error::AppError;
use crate::database::DbError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use color_eyre::eyre;
use serde_json::json;
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid username")]
    InvalidUsername,
    #[error("missing token")]
    MissingToken,
    #[error("invalid token")]
    InvalidToken,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("refresh token expired or not found")]
    RefreshTokenExpiredOrNotFound,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("user not found")]
    UserNotFound,
    #[error("permission denied for {user_email} on {path}")]
    PermissionDenied { user_email: String, path: String },
    #[error("invalid or expired invite token")]
    InvalidInvite,
    #[error(transparent)]
    Internal(#[from] eyre::Report),
}

/// Helper function to log authentication failures with appropriate tracing levels.
fn log_auth_failure(error: &AuthError) {
    match error {
        AuthError::InvalidUsername => info!("Invalid username supplied."),
        AuthError::MissingToken => warn!("Authentication failed: Missing Authorization token."),
        AuthError::InvalidToken => warn!("Authentication failed: Invalid token provided."),
        AuthError::InvalidCredentials => {
            info!("Authentication failed: Invalid credentials provided.");
        }
        AuthError::RefreshTokenExpiredOrNotFound => info!("Refresh token not found or expired."),
        AuthError::UserAlreadyExists => info!("Registration failed: User already exists."),
        AuthError::UserNotFound => warn!("Authentication failed: User from token not found."),
        AuthError::PermissionDenied { user_email, path } => {
            warn!(
                "Authorization failed: User {} tried to access admin endpoint: {}",
                user_email, path
            );
        }
        AuthError::InvalidInvite => info!("Registration failed: Invalid or expired invite token."),
        AuthError::Internal(e) => println!("Error in /auth: {e:?}"),
    }
}

/// Converts an `AuthError` into an Axum `Response`, including logging and a JSON error body.
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        log_auth_failure(&self);

        let (status, error_message) = match self {
            Self::InvalidUsername => (
                StatusCode::BAD_REQUEST,
                "Username may only contain alphanumeric characters and spaces.",
            ),
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid email or password"),
            Self::MissingToken
            | Self::InvalidToken
            | Self::UserNotFound
            | Self::RefreshTokenExpiredOrNotFound => {
                (StatusCode::UNAUTHORIZED, "Authentication failed")
            }
            Self::UserAlreadyExists => (
                StatusCode::CONFLICT,
                "A user with this email already exists",
            ),
            Self::PermissionDenied { .. } => (StatusCode::FORBIDDEN, "Permission denied"),
            Self::InvalidInvite => (StatusCode::FORBIDDEN, "Invalid or expired invite token"),
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal error occurred",
            ),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(eyre::Report::new(err))
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::Internal(eyre::Report::new(err))
    }
}

impl From<DbError> for AuthError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::UniqueViolation(_) => Self::UserAlreadyExists,
            DbError::Sqlx(err) => Self::Internal(eyre::Report::new(err)),
            DbError::SerdeJson(err) => Self::Internal(eyre::Report::new(err)),
        }
    }
}

impl From<AppError> for AuthError {
    fn from(err: AppError) -> Self {
        Self::Internal(eyre::Report::new(err))
    }
}
