use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use axum_extra::extract::Query;
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::explore::interfaces::{
    ExploreTableQuery, HistogramResponse, PaginatedExploreTableResponse,
};
use common_services::api::explore::locations::{get_location, get_visited_places};
use common_services::api::explore::service::{get_explore_table, get_histograms};
use common_types::pb::api::{LocationDetailsResponse, VisitedLocation};
use tracing::instrument;

#[instrument(skip(context, user), err(Debug))]
pub async fn get_explore_table_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(query): Query<ExploreTableQuery>,
) -> Result<Json<PaginatedExploreTableResponse>, AppError> {
    let result = get_explore_table(&context.pool, user.id, query).await?;
    Ok(Json(result))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_histograms_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<HistogramResponse>, AppError> {
    let result = get_histograms(&context.pool, user.id).await?;
    Ok(Json(result))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_visited_places_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<Vec<VisitedLocation>>, AppError> {
    let result = get_visited_places(&context.pool, user.id).await?;
    Ok(Json(result))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_location_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(location_key): Path<String>,
) -> Result<Protobuf<LocationDetailsResponse>, AppError> {
    let result = get_location(&context.pool, user.id, &location_key).await?;
    Ok(Protobuf(result))
}
