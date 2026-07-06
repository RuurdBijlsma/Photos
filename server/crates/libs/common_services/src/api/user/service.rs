use crate::api::app_error::AppError;
use crate::api::user::interfaces::{UserProfile, UserStats};
use crate::database::UpdateField::Value;
use crate::database::app_user::UserRole;
use crate::database::user_store::UserStore;
use crate::database::{UpdateField, UpdateUserPayload};
use sqlx::PgPool;

pub async fn get_user_profile(
    pool: &PgPool,
    logged_in_user_id: i32,
    user_id: i32,
) -> Result<UserProfile, AppError> {
    let user = UserStore::find_by_id(pool, user_id)
        .await?
        .ok_or(AppError::NotFound("user".into()))?;

    let stats = get_user_stats(pool, user_id).await?;

    let email = if logged_in_user_id == user_id || user.role == UserRole::Admin {
        Some(user.email)
    } else {
        None
    };

    Ok(UserProfile {
        id: user.id,
        name: user.name,
        email,
        created_at: user.created_at,
        avatar_id: user.avatar_id,
        stats,
    })
}

pub async fn get_user_stats(pool: &PgPool, user_id: i32) -> Result<UserStats, AppError> {
    let counts = sqlx::query!(
        r#"
        SELECT
            (SELECT COUNT(*) FROM media_item WHERE user_id = $1 AND is_video = false AND deleted = false) as photo_count,
            (SELECT COUNT(*) FROM media_item WHERE user_id = $1 AND is_video = true AND deleted = false) as video_count,
            (SELECT COUNT(*) FROM album WHERE owner_id = $1) as album_count,
            (SELECT COUNT(*) FROM album_collaborator ac JOIN album a ON ac.album_id = a.id WHERE ac.user_id = $1 AND a.owner_id != $1) as shared_album_count
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(UserStats {
        photo_count: counts.photo_count.unwrap_or(0),
        video_count: counts.video_count.unwrap_or(0),
        album_count: counts.album_count.unwrap_or(0),
        shared_album_count: counts.shared_album_count.unwrap_or(0),
    })
}

pub async fn update_user_profile(
    pool: &PgPool,
    user_id: i32,
    name: Option<String>,
    avatar_id: UpdateField<String>,
) -> Result<UserProfile, AppError> {
    if let Value(ref aid) = avatar_id {
        let is_accessible = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM media_item mi
                LEFT JOIN album_media_item ami ON mi.id = ami.media_item_id
                LEFT JOIN album a ON ami.album_id = a.id
                WHERE mi.id = $1 AND (a.is_public = true OR mi.user_id = $2) AND mi.deleted = false
            ) as "exists!"
            "#,
            aid,
            user_id
        )
        .fetch_one(pool)
        .await?;

        if !is_accessible {
            return Err(AppError::BadRequest("Invalid avatar".into()));
        }
    }

    let user = UserStore::update(
        pool,
        user_id,
        UpdateUserPayload {
            name,
            email: None,
            password: None,
            role: None,
            media_folder: None,
            avatar_id,
        },
    )
    .await?;

    let stats = get_user_stats(pool, user_id).await?;

    Ok(UserProfile {
        id: user.id,
        name: user.name,
        email: Some(user.email),
        created_at: user.created_at,
        avatar_id: user.avatar_id,
        stats,
    })
}
