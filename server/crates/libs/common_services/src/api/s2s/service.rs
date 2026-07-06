use crate::api::app_error::AppError;
use crate::database::album::album::AlbumSummary;
use crate::s2s_client::extract_token_claims;
use sqlx::PgPool;
use std::path::Path;
use tracing::instrument;

/// Validates an invitation token and returns the summary of the album.
#[instrument(skip(pool))]
pub async fn get_invite_summary(
    pool: &PgPool,
    token: &str,
    jwt_secret: &str,
) -> Result<AlbumSummary, AppError> {
    let claims = extract_token_claims(token, jwt_secret)?;

    let summary = sqlx::query_as!(
        AlbumSummary,
        r#"
        SELECT
            name,
            description,
            COALESCE(
                (
                    SELECT array_agg(mi.relative_path)
                    FROM album_media_item ami
                        JOIN media_item mi ON ami.media_item_id = mi.id
                    WHERE ami.album_id = $1
                ),
                '{}'
            ) AS "relative_paths!"
        FROM album
        WHERE album.id = $1
        "#,
        claims.sub
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Forbidden("Token invalid".to_owned()))?;
    Ok(summary)
}

/// Validates a token and checks that a `media_item_id` belongs to the token's album.
/// This is a critical security check.
#[instrument(skip(pool))]
pub async fn validate_token_for_media_item(
    pool: &PgPool,
    token: &str,
    jwt_secret: &str,
    media_item_id: &str,
) -> Result<String, AppError> {
    let claims = extract_token_claims(token, jwt_secret)?;

    let album_id = sqlx::query_scalar!(
        r#"
        SELECT album_id
        FROM album_media_item ami WHERE album_id = $1
        AND media_item_id = $2
        "#,
        claims.sub,
        media_item_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Forbidden("Denied".to_owned()))?;

    Ok(album_id)
}

/// Retrieves the relative path for a given media item owned by a user.
#[instrument(skip(pool))]
pub async fn get_media_item_path(
    pool: &PgPool,
    media_root: &Path,
    media_item_id: &str,
) -> Result<std::path::PathBuf, AppError> {
    // todo: deze join is nutteloos
    let relative_path = sqlx::query_scalar!(
        r#"
        SELECT mi.relative_path
        FROM media_item mi
        JOIN app_user u ON mi.user_id = u.id
        WHERE mi.id = $1
        "#,
        media_item_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Media item {media_item_id} not found.")))?;

    Ok(media_root.join(relative_path))
}
