use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::{Path, State};
use axum_extra::extract::Query;
use axum_extra::protobuf::Protobuf;
use axum::{Extension, Json};
use common_services::api::app_error::AppError;
use common_services::api::explore::interfaces::{
    ExploreTableQuery, HistogramResponse, PaginatedExploreTableResponse, VisitedLocation, VisitedPlacesResponse,
};
use common_services::api::explore::service::{
    get_explore_table, get_histograms,
};
use common_types::pb::api::OrderedMediaResponse;
use tracing::instrument;
use common_services::api::explore::locations::{get_location_details, get_location_media, get_visited_places};

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
) -> Result<Json<VisitedPlacesResponse>, AppError> {
    let result = get_visited_places(&context.pool, user.id).await?;
    Ok(Json(result))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_location_media_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(location_id): Path<i32>,
) -> Result<Protobuf<OrderedMediaResponse>, AppError> {
    let items = get_location_media(&context.pool, user.id, location_id).await?;
    Ok(Protobuf(OrderedMediaResponse { items }))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_location_details_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(location_id): Path<i32>,
) -> Result<Json<VisitedLocation>, AppError> {
    let result = get_location_details(&context.pool, user.id, location_id).await?;
    Ok(Json(result))
}