use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::SysRng;

/// Verify a password against a given hash.
/// # Errors
///
/// * `PasswordHash::new` can return an error if the hash string is invalid.
/// * `Argon2::verify_password` can return an error if the password verification fails.
pub fn verify_password(password: &[u8], hash: &str) -> color_eyre::Result<bool> {
    let parsed_hash = PasswordHash::new(hash)?;
    let verified = Argon2::default()
        .verify_password(password, &parsed_hash)
        .is_ok();
    Ok(verified)
}

/// Hash a password using Argon2.
/// # Errors
///
/// * `SaltString::try_from_rng` can return an error if a random salt cannot be generated.
/// * `Argon2::hash_password` can return an error if the password hashing fails.
pub fn hash_password(password: &[u8]) -> color_eyre::Result<String> {
    let argon2 = Argon2::default();
    let salt = SaltString::try_from_rng(&mut SysRng)?;
    let password_hash = argon2.hash_password(password, &salt)?.to_string();
    Ok(password_hash)
}
