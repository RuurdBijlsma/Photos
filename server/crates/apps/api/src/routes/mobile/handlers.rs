use crate::auth::middlewares::user::ApiUser;
use axum::Extension;
use axum::extract::{Query, State};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::mobile::interfaces::TimelineSyncParams;
use common_services::api::mobile::service::get_timeline_sync;
use common_types::pb::api::TimelineSyncResponse;
use sqlx::PgPool;
use tracing::instrument;

/// Compact timeline sync for mobile clients: full snapshot or delta since a previous sync.
///
/// # Errors
///
/// Returns a `AppError` if `since` is not a valid timestamp or the database query fails.
#[instrument(skip(pool, user), err(Debug))]
pub async fn get_timeline_sync_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<TimelineSyncParams>,
) -> Result<Protobuf<TimelineSyncResponse>, AppError> {
    let response = get_timeline_sync(user.id, &pool, params.since).await?;
    Ok(Protobuf(response))
}
