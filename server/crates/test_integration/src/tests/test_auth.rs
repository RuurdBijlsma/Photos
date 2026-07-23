use crate::runner::context::test_context::TestContext;
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

    let response = context
        .http_client
        .get(me_url)
        .send()
        .await?;
    let me_status = response.status();
    let user: User = response.json().await?;

    // ASSERT
    assert_eq!(login_status, reqwest::StatusCode::OK);
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

    // 1. Login to set cookie
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
    // 2. Refresh session using stored cookie
    let response = context
        .http_client
        .post(refresh_url)
        .send()
        .await?;

    let status = response.status();

    // ASSERT
    assert_eq!(status, reqwest::StatusCode::OK);

    // 3. Verify access works with new cookie
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
    // 2. Logout using cookie
    let logout_response = context
        .http_client
        .post(logout_url)
        .send()
        .await?;

    // ASSERT
    assert_eq!(logout_response.status(), reqwest::StatusCode::NO_CONTENT);

    // 3. Verify refresh is unauthorized after logout
    let refresh_response = context
        .http_client
        .post(refresh_url)
        .send()
        .await?;

    assert_eq!(refresh_response.status(), reqwest::StatusCode::UNAUTHORIZED);

    Ok(())
}