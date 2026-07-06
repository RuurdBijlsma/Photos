use crate::runner::context::test_context::TestContext;
use app_state::constants;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use common_services::api::auth::interfaces::{CreateUser, LoginUser, RefreshTokenPayload, Tokens};
use common_services::database::app_user::{User, UserRole};
use common_services::database::user_store::UserStore;
use common_types::dev_constants::{EMAIL, PASSWORD, USERNAME};

pub async fn test_register(context: &TestContext) -> Result<()> {
    // ARRANGE
    let url = format!("{}/auth/register", &context.settings.api.public_url);

    // ACT
    let response = context
        .http_client
        .post(url)
        .json(&CreateUser {
            name: USERNAME.to_owned(),
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
            token: None,
        })
        .send()
        .await?;

    let status = response.status();
    let user: User = response.json().await?;

    // ASSERT
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(user.name, USERNAME);
    assert_eq!(user.email, EMAIL);
    assert_eq!(user.media_folder, None);
    assert_eq!(user.role, UserRole::Admin);

    let all_users = UserStore::list_users(&context.pool).await?;
    assert_eq!(all_users.len(), 1);
    let created_user = &all_users[0];
    assert_eq!(created_user.name, USERNAME);
    assert_eq!(created_user.email, EMAIL);
    assert_eq!(created_user.media_folder, None);
    assert_eq!(created_user.role, UserRole::Admin);

    Ok(())
}

pub async fn test_second_register_attempt(context: &TestContext) -> Result<()> {
    // ARRANGE
    let url = format!("{}/auth/register", &context.settings.api.public_url);

    // ACT
    let response = context
        .http_client
        .post(url)
        .json(&CreateUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
            name: USERNAME.to_owned(),
            token: None,
        })
        .send()
        .await?;
    let status = response.status();

    // ASSERT
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
    let users = UserStore::list_user_ids(&context.pool).await?;
    assert_eq!(users.len(), 1);

    Ok(())
}

pub async fn test_login(context: &TestContext) -> Result<()> {
    // ARRANGE
    let login_url = format!("{}/auth/login", &context.settings.api.public_url);
    let me_url = format!("{}/auth/me", &context.settings.api.public_url);

    // ACT
    let response = context
        .http_client
        .post(login_url)
        .json(&LoginUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
        })
        .send()
        .await?;
    let login_status = response.status();
    let tokens: Tokens = response.json().await?;

    let response = context
        .http_client
        .get(me_url)
        .bearer_auth(tokens.access_token)
        .send()
        .await?;
    let me_status = response.status();
    let user: User = response.json().await?;

    // ASSERT
    assert_eq!(login_status, reqwest::StatusCode::OK);
    let expiry_date =
        DateTime::from_timestamp_secs(tokens.expiry as i64).expect("invalid expiry date");
    let expire_seconds = (expiry_date - Utc::now()).as_seconds_f64();
    let actual_expire_seconds = (constants().auth.access_token_expiry_minutes * 60) as f64;
    assert!(expire_seconds - actual_expire_seconds < 5.);

    assert_eq!(me_status, reqwest::StatusCode::OK);
    assert_eq!(user.name, USERNAME);
    assert_eq!(user.email, EMAIL);
    assert_eq!(user.media_folder, None);
    assert_eq!(user.role, UserRole::Admin);

    Ok(())
}

pub async fn test_refresh(context: &TestContext) -> Result<()> {
    // ARRANGE
    let login_url = format!("{}/auth/login", &context.settings.api.public_url);
    let refresh_url = format!("{}/auth/refresh", &context.settings.api.public_url);
    let me_url = format!("{}/auth/me", &context.settings.api.public_url);

    // 1. Login to get initial tokens
    let response = context
        .http_client
        .post(login_url)
        .json(&LoginUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
        })
        .send()
        .await?;
    let initial_tokens: Tokens = response.json().await?;

    // ACT
    // 2. Use the refresh token to get new tokens
    let response = context
        .http_client
        .post(refresh_url)
        .json(&RefreshTokenPayload {
            refresh_token: initial_tokens.refresh_token,
        })
        .send()
        .await?;

    let status = response.status();
    let new_tokens: Tokens = response.json().await?;

    // ASSERT
    assert_eq!(status, reqwest::StatusCode::OK);

    // 3. Verify the NEW access token works
    let me_response = context
        .http_client
        .get(me_url)
        .bearer_auth(new_tokens.access_token)
        .send()
        .await?;

    assert_eq!(me_response.status(), reqwest::StatusCode::OK);

    Ok(())
}

pub async fn test_logout(context: &TestContext) -> Result<()> {
    // ARRANGE
    let login_url = format!("{}/auth/login", &context.settings.api.public_url);
    let logout_url = format!("{}/auth/logout", &context.settings.api.public_url);
    let refresh_url = format!("{}/auth/refresh", &context.settings.api.public_url);

    // 1. Login
    let response = context
        .http_client
        .post(login_url)
        .json(&LoginUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
        })
        .send()
        .await?;
    let tokens: Tokens = response.json().await?;

    // ACT
    // 2. Logout using the refresh token
    let logout_response = context
        .http_client
        .post(logout_url)
        .json(&RefreshTokenPayload {
            refresh_token: tokens.refresh_token.clone(),
        })
        .send()
        .await?;

    // ASSERT
    assert_eq!(logout_response.status(), reqwest::StatusCode::NO_CONTENT);

    // 3. Verify the refresh token is dead (Try to refresh with it)
    let refresh_response = context
        .http_client
        .post(refresh_url)
        .json(&RefreshTokenPayload {
            refresh_token: tokens.refresh_token,
        })
        .send()
        .await?;

    // Should be 401 Unauthorized because the session was deleted from DB
    assert_eq!(refresh_response.status(), reqwest::StatusCode::UNAUTHORIZED);

    Ok(())
}
