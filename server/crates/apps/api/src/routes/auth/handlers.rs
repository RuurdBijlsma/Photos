use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, extract::State, http::StatusCode};
use common_services::api::app_error::AppError;
use common_services::api::auth::interfaces::{CreateUser, GenerateInvitePayload, LoginUser};
use common_services::api::auth::service::{
    authenticate_user, create_access_token, create_user, generate_invite, logout_user,
    refresh_tokens, store_refresh_token,
};
use common_services::api::auth::token::generate_refresh_token_parts;
use common_services::database::app_user::{User, UserInvite};
use common_services::database::user_store::UserStore;
use sqlx::PgPool;
use tracing::instrument;

/// Formats standard `HttpOnly` cookie properties consistently.
#[must_use]
pub fn make_cookie(name: &str, value: &str, max_age_secs: Option<i64>, secure: bool) -> String {
    let secure_str = if secure { "; Secure" } else { "" };

    max_age_secs.map_or_else(
        || format!("{name}={value}; HttpOnly; Path=/; SameSite=Lax{secure_str}"),
        |max_age| {
            format!("{name}={value}; HttpOnly; Path=/; SameSite=Lax{secure_str}; Max-Age={max_age}")
        },
    )
}

/// Handles user login and returns secure HTTP-only cookies.
#[instrument(skip(context, payload), err(Debug))]
pub async fn login(
    State(context): State<ApiContext>,
    Json(payload): Json<LoginUser>,
) -> Result<Response, AppError> {
    let user = authenticate_user(&context.pool, &payload.email, &payload.password).await?;
    let (access_token, _expiry) =
        create_access_token(&context.settings.secrets.jwt, user.id, user.role)?;
    let token_parts = generate_refresh_token_parts()?;
    store_refresh_token(&context.pool, user.id, &token_parts).await?;

    let secure = context.settings.api.cookie_secure;
    let access_exp = app_state::constants().auth.access_token_expiry_minutes * 60;
    let refresh_exp = app_state::constants().auth.refresh_token_expiry_days * 24 * 3600;

    let access_cookie = make_cookie("access_token", &access_token, Some(access_exp), secure);
    let refresh_cookie = make_cookie(
        "refresh_token",
        &token_parts.raw_token,
        Some(refresh_exp),
        secure,
    );

    let mut response = StatusCode::OK.into_response();
    let headers = response.headers_mut();
    headers.append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&access_cookie)
            .map_err(|_| AppError::Unauthorized("Access cookie missing".to_owned()))?,
    );
    headers.append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&refresh_cookie)
            .map_err(|_| AppError::Unauthorized("Access cookie missing".to_owned()))?,
    );

    Ok(response)
}

/// Handles the registration of a new user.
#[instrument(skip(context, payload), err(Debug))]
pub async fn register(
    State(context): State<ApiContext>,
    Json(payload): Json<CreateUser>,
) -> Result<Json<User>, AppError> {
    let user = create_user(&context.pool, &context.settings.ingest, &payload).await?;
    Ok(Json(user))
}

/// Handles refreshing the session using a valid refresh token cookie.
#[instrument(skip(context, parts), err(Debug))]
pub async fn refresh_session(
    State(context): State<ApiContext>,
    parts: http::request::Parts,
) -> Result<Response, AppError> {
    let refresh_token =
        crate::auth::middlewares::common::extract_cookie_value(&parts, "refresh_token")
            .ok_or_else(|| AppError::Unauthorized("Refresh token missing".to_owned()))?;

    let tokens =
        refresh_tokens(&context.pool, &context.settings.secrets.jwt, &refresh_token).await?;

    let secure = context.settings.api.cookie_secure;
    let access_exp = app_state::constants().auth.access_token_expiry_minutes * 60;
    let refresh_exp = app_state::constants().auth.refresh_token_expiry_days * 24 * 3600;

    let access_cookie = make_cookie(
        "access_token",
        &tokens.access_token,
        Some(access_exp),
        secure,
    );
    let refresh_cookie = make_cookie(
        "refresh_token",
        &tokens.refresh_token,
        Some(refresh_exp),
        secure,
    );

    let mut response = StatusCode::OK.into_response();
    let headers = response.headers_mut();
    headers.append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&access_cookie)
            .map_err(|_| AppError::Unauthorized("Access cookie missing".to_owned()))?,
    );
    headers.append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&refresh_cookie)
            .map_err(|_| AppError::Unauthorized("Access cookie missing".to_owned()))?,
    );

    Ok(response)
}

/// Handles user logout by invalidating the refresh token and clearing cookies.
pub async fn logout(
    State(context): State<ApiContext>,
    parts: http::request::Parts,
) -> Result<Response, AppError> {
    if let Some(refresh_token) =
        crate::auth::middlewares::common::extract_cookie_value(&parts, "refresh_token")
    {
        let _ = logout_user(&context.pool, &refresh_token).await;
    }

    let secure = context.settings.api.cookie_secure;
    let access_cookie = make_cookie("access_token", "", Some(0), secure);
    let refresh_cookie = make_cookie("refresh_token", "", Some(0), secure);

    let mut response = StatusCode::NO_CONTENT.into_response();
    let headers = response.headers_mut();
    headers.append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&access_cookie)
            .map_err(|_| AppError::Unauthorized("Access cookie missing".to_owned()))?,
    );
    headers.append(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&refresh_cookie)
            .map_err(|_| AppError::Unauthorized("Refresh cookie missing".to_owned()))?,
    );

    Ok(response)
}

/// Get current user info.
pub async fn get_me(
    Extension(user): Extension<ApiUser>,
    State(pool): State<PgPool>,
) -> Result<Json<User>, AppError> {
    let Some(user_info) = UserStore::find_by_id(&pool, user.id).await? else {
        return Err(AppError::NotFound("User not found".to_owned()));
    };
    Ok(Json(user_info))
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
