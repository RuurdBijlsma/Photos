use crate::database::DbError;
use crate::database::structs::UpdateUserPayload;
use crate::database::tables::app_user::{User, UserInvite, UserRole, UserWithPassword};
use chrono::{DateTime, Utc};
use sqlx::postgres::PgQueryResult;
use sqlx::{Executor, Postgres};

pub struct UserStore;

impl UserStore {
    //================================================================================
    // Core User Management (CRUD)
    //================================================================================

    /// Creates a new user.
    pub async fn create(
        executor: impl Executor<'_, Database = Postgres>,
        email: &str,
        name: &str,
        hashed_password: &str,
        role: UserRole,
        media_folder: Option<String>,
    ) -> Result<User, DbError> {
        Ok(sqlx::query_as!(
            User,
            r#"
            INSERT INTO app_user (email, name, password, role, media_folder)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id,
                created_at,
                updated_at,
                email,
                name,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            "#,
            email,
            name,
            hashed_password,
            role as UserRole,
            media_folder
        )
        .fetch_one(executor)
        .await?)
    }

    /// Updates a user's details.
    ///
    /// Pass `None` for fields that should remain unchanged.
    pub async fn update(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
        payload: UpdateUserPayload,
    ) -> Result<User, DbError> {
        Ok(sqlx::query_as!(
            User,
            r#"
            UPDATE app_user
            SET
                name = COALESCE($2, name),
                email = COALESCE($3, email),
                password = COALESCE($4, password),
                role = COALESCE($5, role),
                media_folder = COALESCE($6, media_folder),
                avatar_id = CASE WHEN $7::boolean THEN avatar_id ELSE $8 END,
                updated_at = now()
            WHERE id = $1
            RETURNING
                id,
                created_at,
                updated_at,
                email,
                name,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            "#,
            user_id,
            payload.name,
            payload.email,
            payload.password,
            payload.role as Option<UserRole>,
            payload.media_folder,
            payload.avatar_id.is_ignore(),
            payload.avatar_id.value()
        )
        .fetch_one(executor)
        .await?)
    }

    /// Deletes a user by ID.
    pub async fn delete(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
    ) -> Result<PgQueryResult, DbError> {
        Ok(sqlx::query!("DELETE FROM app_user WHERE id = $1", user_id)
            .execute(executor)
            .await?)
    }

    //================================================================================
    // Find / Get Methods
    //================================================================================

    pub async fn find_by_id(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
    ) -> Result<Option<User>, DbError> {
        Ok(sqlx::query_as!(
            User,
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                email,
                name,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            FROM app_user
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn find_by_email(
        executor: impl Executor<'_, Database = Postgres>,
        email: &str,
    ) -> Result<Option<User>, DbError> {
        Ok(sqlx::query_as!(
            User,
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                email,
                name,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            FROM app_user
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn find_by_email_with_password(
        executor: impl Executor<'_, Database = Postgres>,
        email: &str,
    ) -> Result<Option<UserWithPassword>, DbError> {
        Ok(sqlx::query_as!(
            UserWithPassword,
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                email,
                name,
                password,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            FROM app_user
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(executor)
        .await?)
    }

    /// Retrieves just the role for a specific user ID.
    pub async fn get_user_role(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
    ) -> Result<Option<UserRole>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT role as "role: UserRole"
            FROM app_user
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(executor)
        .await?)
    }

    /// Retrieves just the `media_folder` for a specific user ID.
    pub async fn get_user_media_folder(
        executor: impl Executor<'_, Database = Postgres>,
        user_id: i32,
    ) -> Result<Option<String>, DbError> {
        Ok(sqlx::query_scalar!(
            r#"
            SELECT media_folder
            FROM app_user
            WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(executor)
        .await?
        .flatten())
    }

    /// Derives the user ID from a given relative path by extracting the username and querying the database.
    /// # Errors
    ///
    /// * If the username cannot be extracted from the path.
    /// * If the database query to find the user by username fails.
    /// * If no user is found for the extracted username.
    pub async fn find_user_by_relative_path(
        executor: impl Executor<'_, Database = Postgres>,
        relative_path: &str,
    ) -> color_eyre::Result<Option<User>> {
        let users = Self::list_users_with_media_folders(executor).await?;

        let mut best_match: Option<User> = None;
        let mut max_len = -1;

        for user in users {
            if let Some(media_folder) = &user.media_folder
                && relative_path.starts_with(media_folder)
                && media_folder.len() as i32 > max_len
            {
                max_len = media_folder.len() as i32;
                best_match = Some(user);
            }
        }

        Ok(best_match)
    }

    //================================================================================
    // Utilities
    //================================================================================

    /// Lists all users who have a media folder configured.
    /// Useful for mapping file system paths to users.
    pub async fn list_users_with_media_folders(
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Vec<User>, DbError> {
        Ok(sqlx::query_as!(
            User,
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                email,
                name,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            FROM app_user
            WHERE media_folder IS NOT NULL
            "#,
        )
        .fetch_all(executor)
        .await?)
    }

    pub async fn list_users(
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Vec<User>, DbError> {
        Ok(sqlx::query_as!(
            User,
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                email,
                name,
                media_folder,
                avatar_id,
                role as "role: UserRole"
            FROM app_user
            "#,
        )
        .fetch_all(executor)
        .await?)
    }

    pub async fn list_user_ids(
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Vec<i32>, DbError> {
        Ok(sqlx::query_scalar!(r#"SELECT id FROM app_user"#)
            .fetch_all(executor)
            .await?)
    }

    //================================================================================
    // User Invites
    //================================================================================

    pub async fn create_invite(
        executor: impl Executor<'_, Database = Postgres>,
        token: &str,
        media_folder: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<UserInvite, DbError> {
        Ok(sqlx::query_as!(
            UserInvite,
            r#"
            INSERT INTO user_invite (token, media_folder, expires_at)
            VALUES ($1, $2, $3)
            RETURNING token, media_folder, expires_at, created_at
            "#,
            token,
            media_folder,
            expires_at
        )
        .fetch_one(executor)
        .await?)
    }

    pub async fn find_invite_by_token(
        executor: impl Executor<'_, Database = Postgres>,
        token: &str,
    ) -> Result<Option<UserInvite>, DbError> {
        Ok(sqlx::query_as!(
            UserInvite,
            r#"
            SELECT token, media_folder, expires_at, created_at
            FROM user_invite
            WHERE token = $1 AND expires_at > NOW()
            "#,
            token
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn delete_invite(
        executor: impl Executor<'_, Database = Postgres>,
        token: &str,
    ) -> Result<PgQueryResult, DbError> {
        Ok(
            sqlx::query!("DELETE FROM user_invite WHERE token = $1", token)
                .execute(executor)
                .await?,
        )
    }
}
