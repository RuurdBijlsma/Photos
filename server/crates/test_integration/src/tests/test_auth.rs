use crate::runner::context::test_context::TestContext;
use app_state::constants;
use color_eyre::Result;
use common_services::api::auth::interfaces::{CreateUser, LoginUser};
use common_services::database::app_user::{User, UserRole};
use common_services::database::user_store::UserStore;
use common_types::dev_constants::{EMAIL, PASSWORD, USERNAME};

pub async fn test_register(context: &TestContext) -> Result<()> {
    // ARRANGE
    let url = format!("{}/auth/register", context.settings.api.public_url);

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
    let url = format!("{}/auth/register", context.settings.api.public_url);

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
    let login_url = format!("{}/auth/login", context.settings.api.public_url);
    let me_url = format!("{}/auth/me", context.settings.api.public_url);

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

    // Extract access_token cookie to assert expiry properties
    let set_cookie_str = response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .find(|s| s.starts_with("access_token="))
        .ok_or_else(|| color_eyre::eyre::eyre!("access_token cookie not found"))?;

    let max_age_secs = set_cookie_str
        .split(';')
        .map(str::trim)
        .find(|part| part.starts_with("Max-Age="))
        .and_then(|part| part.split('=').nth(1))
        .and_then(|val| val.parse::<f64>().ok())
        .ok_or_else(|| color_eyre::eyre::eyre!("Max-Age attribute not found in cookie"))?;

    let response = context
        .http_client
        .get(me_url)
        .send()
        .await?;
    let me_status = response.status();
    let user: User = response.json().await?;

    // ASSERT
    assert_eq!(login_status, reqwest::StatusCode::OK);

    let actual_expire_seconds = (constants().auth.access_token_expiry_minutes * 60) as f64;
    assert!((max_age_secs - actual_expire_seconds).abs() < 5.);

    assert_eq!(me_status, reqwest::StatusCode::OK);
    assert_eq!(user.name, USERNAME);
    assert_eq!(user.email, EMAIL);
    assert_eq!(user.media_folder, None);
    assert_eq!(user.role, UserRole::Admin);

    Ok(())
}

pub async fn test_refresh(context: &TestContext) -> Result<()> {
    // ARRANGE
    let login_url = format!("{}/auth/login", context.settings.api.public_url);
    let refresh_url = format!("{}/auth/refresh", context.settings.api.public_url);
    let me_url = format!("{}/auth/me", context.settings.api.public_url);

    // 1. Login to populate cookie jar
    let response = context
        .http_client
        .post(login_url)
        .json(&LoginUser {
            email: EMAIL.to_owned(),
            password: PASSWORD.to_owned(),
        })
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // ACT
    // 2. Call refresh (cookies are sent and updated in the background)
    let response = context
        .http_client
        .post(refresh_url)
        .send()
        .await?;

    let status = response.status();

    // ASSERT
    assert_eq!(status, reqwest::StatusCode::OK);

    // 3. Verify the updated access token works
    let me_response = context
        .http_client
        .get(me_url)
        .send()
        .await?;

    assert_eq!(me_response.status(), reqwest::StatusCode::OK);

    Ok(())
}

pub async fn test_logout(context: &TestContext) -> Result<()> {
    // ARRANGE
    let login_url = format!("{}/auth/login", context.settings.api.public_url);
    let logout_url = format!("{}/auth/logout", context.settings.api.public_url);
    let refresh_url = format!("{}/auth/refresh", context.settings.api.public_url);

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
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // ACT
    // 2. Logout (this requests deletion and prompts the client to clear cookies)
    let logout_response = context
        .http_client
        .post(logout_url)
        .send()
        .await?;

    // ASSERT
    assert_eq!(logout_response.status(), reqwest::StatusCode::NO_CONTENT);

    // 3. Verify the refresh token is dead (Try to refresh with cleared cookies)
    let refresh_response = context
        .http_client
        .post(refresh_url)
        .send()
        .await?;

    // Should return 401 Unauthorized because the refresh token cookie has been removed/invalidated
    assert_eq!(refresh_response.status(), reqwest::StatusCode::UNAUTHORIZED);

    Ok(())
}