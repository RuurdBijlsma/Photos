use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::system::storage::{
    get_blurry_storage_items, get_large_storage_items,
    get_missing_storage_items, get_storage_summary, prune_all_missing_items,
    prune_single_missing_item,
};
use common_types::pb::api::{StorageReviewResponse, StorageSummaryResponse};
use sqlx::PgPool;
use tracing::instrument;
use common_services::api::system::interfaces::PruneMissingResponse;

#[instrument(skip(context, user), err(Debug))]
pub async fn storage_summary_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Protobuf<StorageSummaryResponse>, AppError> {
    Ok(Protobuf(
        get_storage_summary(&context.pool, &context.settings.ingest, user.id).await?,
    ))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn storage_review_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Protobuf<StorageReviewResponse>, AppError> {
    Ok(Protobuf(get_large_storage_items(&pool, user.id).await?))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn storage_blurry_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Protobuf<StorageReviewResponse>, AppError> {
    Ok(Protobuf(get_blurry_storage_items(&pool, user.id).await?))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn storage_missing_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Protobuf<StorageReviewResponse>, AppError> {
    Ok(Protobuf(get_missing_storage_items(&pool, user.id).await?))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn prune_all_missing_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<PruneMissingResponse>, AppError> {
    let pruned_count =
        prune_all_missing_items(&context.pool, &context.settings.ingest, user.id).await?;
    Ok(Json(PruneMissingResponse { pruned_count }))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn prune_single_missing_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(media_item_id): Path<String>,
) -> Result<StatusCode, AppError> {
    prune_single_missing_item(
        &context.pool,
        &context.settings.ingest,
        user.id,
        &media_item_id,
    )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}