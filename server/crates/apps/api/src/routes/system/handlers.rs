use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::State;
use axum::{Extension, Json};
use common_services::api::app_error::AppError;
use common_services::api::system::interfaces::{PublicSystemStats, SystemStats};
use common_services::api::system::service::{get_public_system_stats, get_system_stats};

pub async fn get_system_stats_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<SystemStats>, AppError> {
    let stats = get_system_stats(&context.pool, &context.settings.ingest, user.id).await?;
    Ok(Json(stats))
}

pub async fn get_public_system_stats_handler(
    State(context): State<ApiContext>,
) -> Result<Json<PublicSystemStats>, AppError> {
    let stats = get_public_system_stats(&context.pool).await?;
    Ok(Json(stats))
}
