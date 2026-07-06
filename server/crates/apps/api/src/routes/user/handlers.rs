use crate::api_state::ApiContext;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use common_services::api::app_error::AppError;
use common_services::api::user::interfaces::{SmallUser, UpdateUserProfileRequest, UserProfile};
use common_services::api::user::service::{get_user_profile, update_user_profile};
use common_services::database::app_user::User;
use common_services::database::user_store::UserStore;

/// Fetch the profile data and library statistics for any user.
pub async fn get_user_profile_handler(
    State(ctx): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(user_id): Path<i32>,
) -> Result<Json<UserProfile>, AppError> {
    let profile = get_user_profile(&ctx.pool, user.id, user_id).await?;
    Ok(Json(profile))
}

/// Update the current authenticated user's settings.
pub async fn update_my_profile(
    State(ctx): State<ApiContext>,
    Extension(user): Extension<User>,
    Json(payload): Json<UpdateUserProfileRequest>,
) -> Result<Json<UserProfile>, AppError> {
    let profile = update_user_profile(&ctx.pool, user.id, payload.name, payload.avatar_id).await?;
    Ok(Json(profile))
}

pub async fn list_users_handler(
    State(ctx): State<ApiContext>,
) -> Result<Json<Vec<SmallUser>>, AppError> {
    let users = UserStore::list_users(&ctx.pool).await?;
    Ok(Json(users.into_iter().map(Into::into).collect()))
}
