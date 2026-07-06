//! This module defines the HTTP handlers for authentication-related routes.

use crate::api_state::ApiContext;
use axum::{Extension, Json, extract::State, http::StatusCode};

use common_services::api::app_error::AppError;
use common_services::api::auth::interfaces::{
    CreateUser, GenerateInvitePayload, LoginUser, RefreshTokenPayload, Tokens,
};
use common_services::api::auth::service::{
    authenticate_user, create_access_token, create_user, generate_invite, logout_user,
    refresh_tokens, store_refresh_token,
};
use common_services::api::auth::token::generate_refresh_token_parts;
use common_services::database::app_user::{User, UserInvite};
use tracing::instrument;

/// Handles user login and returns a new set of tokens.
///
/// # Errors
///
/// Returns `AppError` if the user credentials are invalid or if there's a
/// problem creating or storing the tokens.
#[instrument(skip(context, payload), err(Debug))]
pub async fn login(
    State(context): State<ApiContext>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<Tokens>, AppError> {
    let user = authenticate_user(&context.pool, &payload.email, &payload.password).await?;
    let (access_token, expiry) =
        create_access_token(&context.settings.secrets.jwt, user.id, user.role)?;
    let token_parts = generate_refresh_token_parts()?;
    store_refresh_token(&context.pool, user.id, &token_parts).await?;

    Ok(Json(Tokens {
        expiry,
        access_token,
        refresh_token: token_parts.raw_token,
    }))
}

/// Handles the registration of a new user.
///
/// # Errors
///
/// Returns `AppError` if a user with the provided email already exists or
/// if a database error occurs during user creation.
#[instrument(skip(context, payload), err(Debug))]
pub async fn register(
    State(context): State<ApiContext>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    let user = create_user(&context.pool, &context.settings.ingest, &payload).await?;
    Ok(Json(user))
}

/// Handles refreshing the session using a valid refresh token.
///
/// # Errors
///
/// Returns `AppError` if the refresh token is invalid, expired, or not found in the database.
#[instrument(skip(context, payload), err(Debug))]
pub async fn refresh_session(
    State(context): State<ApiContext>,
    Json(payload): Json<RefreshTokenPayload>,
) -> Result<Json<Tokens>, AppError> {
    refresh_tokens(
        &context.pool,
        &context.settings.secrets.jwt,
        &payload.refresh_token,
    )
    .await
}

/// Handles user logout by invalidating the provided refresh token.
///
/// # Errors
///
/// Returns `AppError` if the refresh token is invalid or could not be found.
pub async fn logout(
    State(context): State<ApiContext>,
    Json(payload): Json<RefreshTokenPayload>,
) -> Result<StatusCode, AppError> {
    logout_user(&context.pool, &payload.refresh_token).await
}

/// Get current user info.
///
/// # Errors
///
/// Returns `AppError` if the refresh token is invalid or could not be found.
pub async fn get_me(Extension(user): Extension<User>) -> Result<Json<User>, StatusCode> {
    Ok(Json(user))
}

/// Generates a new user invite token.
pub async fn generate_invite_handler(
    State(context): State<ApiContext>,
    Json(payload): Json<GenerateInvitePayload>,
) -> Result<Json<UserInvite>, AppError> {
    let invite = generate_invite(
        &context.pool,
        &context.settings.ingest,
        &payload.user_folder,
    )
    .await?;
    Ok(Json(invite))
}
