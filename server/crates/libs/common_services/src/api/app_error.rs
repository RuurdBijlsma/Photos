use crate::api::auth::error::AuthError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use color_eyre::eyre::eyre;
use image::ImageError;
use material_color_utils::utils::error::ColorParseError;
use open_clip_inference::ClipError;
use serde_json::json;
use tracing::{error, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),

    #[error("Internal server error")]
    Internal(#[from] color_eyre::Report),

    #[error("Range not satisfiable: {message}")]
    RangeNotSatisfiable { message: String, file_size: u64 },
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(msg) => {
                info!(target: "api", "Bad Request: {}", msg);
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            }
            Self::Unauthorized(msg) => {
                info!(target: "api", "Unauthorized access: {}", msg);
                (StatusCode::UNAUTHORIZED, Json(json!({ "error": msg }))).into_response()
            }
            Self::Forbidden(msg) => {
                warn!(target: "api", "Forbidden access: {}", msg);
                (StatusCode::FORBIDDEN, Json(json!({ "error": msg }))).into_response()
            }
            Self::NotFound(msg) => {
                info!(target: "api", "Not found: {}", msg);
                (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))).into_response()
            }
            Self::Conflict(msg) => {
                warn!(target: "api", "Conflict: {}", msg);
                (StatusCode::CONFLICT, Json(json!({ "error": msg }))).into_response()
            }
            Self::UnsupportedMediaType(msg) => {
                warn!(target: "api", "Unsupported media type: {}", msg);
                (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    Json(json!({ "error": msg })),
                )
                    .into_response()
            }
            Self::RangeNotSatisfiable { message, file_size } => {
                info!(target: "api", "Range not satisfiable: {} (File size: {})", message, file_size);
                let mut response = (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    Json(json!({ "error": message })),
                )
                    .into_response();
                if let Ok(header_val) = http::HeaderValue::from_str(&format!("bytes */{file_size}"))
                {
                    response
                        .headers_mut()
                        .insert(http::header::CONTENT_RANGE, header_val);
                }
                response
            }
            Self::Internal(err) => {
                error!(target: "api", "Internal Server Error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "An unexpected internal error occurred." })),
                )
                    .into_response()
            }
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        if matches!(err, sqlx::Error::RowNotFound) {
            Self::NotFound("The requested resource was not found.".to_string())
        } else {
            Self::Internal(color_eyre::Report::new(err))
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(color_eyre::Report::new(err))
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Internal(color_eyre::Report::new(err))
    }
}

impl From<crate::database::DbError> for AppError {
    fn from(err: crate::database::DbError) -> Self {
        match err {
            crate::database::DbError::UniqueViolation(_) => {
                Self::Conflict("A record with this unique attribute already exists.".to_string())
            }
            crate::database::DbError::Sqlx(e) => Self::from(e),
            crate::database::DbError::SerdeJson(e) => Self::Internal(color_eyre::Report::new(e)),
        }
    }
}

impl From<chrono::ParseError> for AppError {
    fn from(err: chrono::ParseError) -> Self {
        Self::BadRequest(format!("Invalid datetime format: {err}"))
    }
}

impl From<ColorParseError> for AppError {
    fn from(err: ColorParseError) -> Self {
        Self::BadRequest(format!("Color parse error: {err}"))
    }
}

impl From<ClipError> for AppError {
    fn from(err: ClipError) -> Self {
        Self::Internal(eyre!(err))
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::Internal(eyre!(err))
    }
}

impl From<url::ParseError> for AppError {
    fn from(err: url::ParseError) -> Self {
        Self::Internal(eyre!(err))
    }
}

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        Self::Internal(eyre!(err))
    }
}

impl From<ImageError> for AppError {
    fn from(err: ImageError) -> Self {
        Self::Internal(eyre!(err))
    }
}
