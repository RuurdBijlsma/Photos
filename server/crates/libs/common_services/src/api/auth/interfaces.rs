use crate::database::app_user::UserRole;
use serde::{Deserialize, Serialize};

/// Represents the data required to create a new user.
#[derive(Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUser {
    pub email: String,
    pub name: String,
    pub password: String,
    pub token: Option<String>,
}

/// Represents the data required for user login.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoginUser {
    pub email: String,
    pub password: String,
}

/// Represents the payload for a refresh token request.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenPayload {
    pub refresh_token: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GenerateInvitePayload {
    pub user_folder: String,
}

/// Represents a pair of access and refresh tokens.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    pub expiry: u64,
    pub access_token: String,
    pub refresh_token: String,
}

/// Represents the claims contained within a JWT.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AuthClaims {
    pub sub: i32, // Subject (user ID)
    pub exp: i64, // Expiration time
    pub role: UserRole,
}
