use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::State;
use axum::{Extension, Json};
use common_services::api::app_error::AppError;
use common_services::api::system::interfaces::SystemStats;
use common_services::api::system::service::get_system_stats;

pub async fn get_system_stats_handler(
    State(ctx): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<SystemStats>, AppError> {
    let stats = get_system_stats(&ctx.pool, &ctx.settings.ingest, user.id).await?;
    Ok(Json(stats))
}
