use crate::api::admin::service::{check_folder_in_use, is_valid_user_folder};
use crate::api::app_error::AppError;
use crate::api::auth::hashing::{hash_password, verify_password};
use crate::api::auth::interfaces::{AuthClaims, CreateUser, Tokens};
use crate::api::auth::token::{
    RefreshTokenParts, generate_refresh_token_parts, split_refresh_token, verify_token,
};
use crate::database::app_user::{User, UserInvite, UserRole, UserWithPassword};
use crate::database::user_store::UserStore;
use crate::job_queue::enqueue_full_scan;
use crate::utils::nice_id;
use app_state::{IngestSettings, MakeRelativePath, constants};
use axum::Json;
use axum::http::StatusCode;
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::{Executor, PgPool, Postgres};
use tracing::info;

/// Authenticates a user based on email and password.
pub async fn authenticate_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<UserWithPassword, AppError> {
    let user = UserStore::find_by_email_with_password(pool, email)
        .await?
        .ok_or(AppError::Unauthorized("Invalid credentials".to_owned()))?;

    let valid = verify_password(password.as_ref(), &user.password)?;
    if !valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_owned()));
    }

    Ok(user)
}

/// Creates a new user in the database.
pub async fn create_user(
    pool: &PgPool,
    settings: &IngestSettings,
    payload: &CreateUser,
) -> Result<User, AppError> {
    let username = &payload.name;
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '_' || c == '-')
        || username.starts_with(' ')
        || username.ends_with(' ')
    {
        return Err(AppError::BadRequest("Invalid username".to_owned()));
    }
    let hashed = hash_password(payload.password.as_ref())?;
    info!(
        "Creating user email={}, name={}",
        payload.email, payload.name
    );
    let is_first_user = sqlx::query_scalar!("SELECT 1 FROM app_user")
        .fetch_optional(pool)
        .await?
        .flatten()
        .is_none();

    if is_first_user {
        Ok(UserStore::create(
            pool,
            &payload.email,
            &payload.name,
            &hashed,
            UserRole::Admin,
            None,
        )
        .await?)
    } else if let Some(token) = &payload.token {
        let Some(invite) = UserStore::find_invite_by_token(pool, token).await? else {
            return Err(AppError::Unauthorized("Invalid Invite".to_owned()));
        };
        let user = UserStore::create(
            pool,
            &payload.email,
            &payload.name,
            &hashed,
            UserRole::User,
            Some(invite.media_folder),
        )
        .await?;
        UserStore::delete_invite(pool, token).await?;
        enqueue_full_scan(pool, settings, user.id).await?;
        Ok(user)
    } else {
        Err(AppError::Unauthorized("Invalid Invite".to_owned()))
    }
}

pub async fn generate_invite(
    pool: &PgPool,
    ingest_settings: &IngestSettings,
    user_folder: &str,
) -> Result<UserInvite, AppError> {
    let token = nice_id(32);
    let expires_at = Utc::now() + Duration::days(7);
    let user_folder = is_valid_user_folder(&ingest_settings.media_root, user_folder).await?;
    let relative = user_folder.make_relative_canon(&ingest_settings.media_root_canon)?;
    if check_folder_in_use(pool, &relative, None).await? {
        return Err(AppError::BadRequest("Folder already in use".to_owned()));
    }
    Ok(UserStore::create_invite(pool, &token, &relative, expires_at).await?)
}

/// Stores a refresh token in the database.
pub async fn store_refresh_token<'c, E>(
    executor: E,
    user_id: i32,
    parts: &RefreshTokenParts,
) -> Result<(), AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    let exp = Utc::now() + Duration::days(constants().auth.refresh_token_expiry_days);
    sqlx::query!(
        "INSERT INTO refresh_token (user_id, selector, verifier_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
        user_id,
        parts.selector,
        parts.verifier_hash,
        exp
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Creates a new access token for a given user ID and role.
pub fn create_access_token(
    jwt_secret: &str,
    user_id: i32,
    role: UserRole,
) -> Result<(String, u64), AppError> {
    let exp =
        (Utc::now() + Duration::minutes(constants().auth.access_token_expiry_minutes)).timestamp();
    let claims = AuthClaims {
        sub: user_id,
        role,
        exp,
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(Into::<AppError>::into)?;

    Ok((access_token, exp as u64))
}

/// Handles refresh token rotation, invalidating the old token and issuing a new pair.
pub async fn refresh_tokens(
    pool: &PgPool,
    jwt_secret: &str,
    raw_token: &str,
) -> Result<Json<Tokens>, AppError> {
    let (selector, verifier_bytes) = split_refresh_token(raw_token)?;
    let record = sqlx::query!(
        "SELECT user_id, verifier_hash FROM refresh_token
         WHERE selector = $1 AND expires_at > NOW()",
        selector
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized(
        "Refresh token expired or not found".to_owned(),
    ))?;

    if !verify_token(&verifier_bytes, &record.verifier_hash)? {
        // If the verifier is wrong, assume token theft and delete all refresh tokens for that user.
        sqlx::query!(
            "DELETE FROM refresh_token WHERE user_id = $1",
            record.user_id
        )
        .execute(pool)
        .await
        .ok(); // Ignore error if deletion fails
        return Err(AppError::Unauthorized("Invalid token".to_owned()));
    }

    let user_role = UserStore::get_user_role(pool, record.user_id)
        .await?
        .ok_or(AppError::NotFound("User not found".to_owned()))?;

    let mut tx = pool.begin().await?;
    sqlx::query!("DELETE FROM refresh_token WHERE selector = $1", selector)
        .execute(&mut *tx)
        .await?;

    let new_parts = generate_refresh_token_parts()?;
    store_refresh_token(&mut *tx, record.user_id, &new_parts).await?;

    tx.commit().await?;

    let (access_token, expiry) = create_access_token(jwt_secret, record.user_id, user_role)?;
    Ok(Json(Tokens {
        expiry,
        access_token,
        refresh_token: new_parts.raw_token,
    }))
}

/// Deletes the refresh token matching the provided one, effectively logging out the user.
pub async fn logout_user(pool: &PgPool, raw_token: &str) -> Result<StatusCode, AppError> {
    // If the token is malformed, we just ignore it and succeed silently.
    if let Ok((selector, verifier_bytes)) = split_refresh_token(raw_token)
        && let Some(rec) = sqlx::query!(
            "SELECT user_id, verifier_hash
            FROM refresh_token
            WHERE selector = $1",
            selector
        )
        .fetch_optional(pool)
        .await?
        && verify_token(&verifier_bytes, &rec.verifier_hash).unwrap_or(false)
    {
        sqlx::query!("DELETE FROM refresh_token WHERE selector = $1", selector)
            .execute(pool)
            .await?;
    }
    // Logout should always appear successful to prevent token enumeration attacks.
    Ok(StatusCode::NO_CONTENT)
}
