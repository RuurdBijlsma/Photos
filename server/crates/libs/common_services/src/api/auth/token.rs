use crate::api::app_error::AppError;
use crate::api::auth::hashing::{hash_password, verify_password};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, rng};

/// Represents the components of a refresh token for secure storage and verification.
pub struct RefreshTokenParts {
    pub raw_token: String,
    pub selector: String,
    pub verifier_hash: String,
}

/// Generates a new set of refresh token parts: a raw token, a selector, and a verifier hash.
pub fn generate_refresh_token_parts() -> Result<RefreshTokenParts, AppError> {
    let mut raw_bytes = [0u8; 32];
    rng().fill_bytes(&mut raw_bytes);

    let selector_bytes = &raw_bytes[..16];
    let verifier_bytes = &raw_bytes[16..];

    let selector = URL_SAFE_NO_PAD.encode(selector_bytes);
    let raw_token = URL_SAFE_NO_PAD.encode(raw_bytes);
    let verifier_hash = hash_password(verifier_bytes)?;

    Ok(RefreshTokenParts {
        raw_token,
        selector,
        verifier_hash,
    })
}

/// Splits a raw refresh token string into its selector and verifier bytes.
pub fn split_refresh_token(token: &str) -> Result<(String, Vec<u8>), AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| AppError::Unauthorized("Invalid token".to_owned()))?;

    if bytes.len() != 32 {
        return Err(AppError::Unauthorized("Invalid token".to_owned()));
    }

    let selector = URL_SAFE_NO_PAD.encode(&bytes[..16]);
    Ok((selector, bytes[16..].to_vec()))
}

/// Verifies a token's verifier bytes against a stored verifier hash.
///
/// # Errors
///
/// * `AppError::Internal` if password verification fails internally.
pub fn verify_token(verifier_bytes: &[u8], verifier_hash: &str) -> Result<bool, AppError> {
    Ok(verify_password(verifier_bytes, verifier_hash)?)
}
