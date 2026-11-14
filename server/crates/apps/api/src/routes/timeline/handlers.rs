use crate::auth::middlewares::user::ApiUser;
use axum::extract::{Query, State};
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use chrono::NaiveDate;
use common_services::api::app_error::AppError;
use common_services::api::timeline::interfaces::{
    GetMediaByMonthParams, TimelineParams, TimelineSyncParams,
};
use common_services::api::timeline::service::{
    get_photos_by_month, get_timeline_ids, get_timeline_ratios,
};
use common_services::api::timeline::sync::get_timeline_sync;
use common_types::pb::api::{TimelineItemsResponse, TimelineRatiosResponse, TimelineSyncResponse};
use sqlx::PgPool;
use tracing::instrument;

/// Get a timeline of all media ratios, grouped by month.
///
/// # Errors
///
/// Returns a `AppError` if the database query fails.
pub async fn get_timeline_ratios_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<TimelineParams>,
) -> Result<Protobuf<TimelineRatiosResponse>, AppError> {
    let timeline = get_timeline_ratios(user.id, &pool, params.sort).await?;
    Ok(Protobuf(timeline))
}

/// Get a timeline of all media ids
///
/// # Errors
///
/// Returns a `AppError` if the database query fails.
pub async fn get_timeline_ids_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<TimelineParams>,
) -> Result<Json<Vec<String>>, AppError> {
    let timeline = get_timeline_ids(user.id, &pool, params.sort).await?;
    Ok(Json(timeline))
}

/// Get all media items for a given set of months.
///
/// # Errors
///
/// Returns a `AppError` if the database query fails.
pub async fn get_photos_by_month_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<GetMediaByMonthParams>,
) -> Result<Protobuf<TimelineItemsResponse>, AppError> {
    let month_ids = params
        .months
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|date_str| NaiveDate::parse_from_str(date_str, "%Y-%m-%d"))
        .collect::<Result<Vec<NaiveDate>, _>>()
        .map_err(|_| {
            AppError::BadRequest(
                "Invalid date format in 'months' parameter. Please use 'YYYY-MM-DD'.".to_string(),
            )
        })?;

    let photos = get_photos_by_month(user.id, &pool, &month_ids, params.sort).await?;
    Ok(Protobuf(photos))
}

/// Compact timeline sync: full snapshot, or the items changed and ids deleted since a previous sync.
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
