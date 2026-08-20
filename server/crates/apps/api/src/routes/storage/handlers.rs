use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::State;
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::system::interfaces::{PruneMissingRequest, PruneMissingResponse};
use common_services::api::system::storage::{
    get_blurry_storage_items, get_large_storage_items, get_missing_storage_items,
    get_storage_summary, prune_missing_items,
};
use common_types::pb::api::{StorageReviewResponse, StorageSummaryResponse};
use sqlx::PgPool;
use tracing::instrument;

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
pub async fn prune_missing_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<PruneMissingRequest>,
) -> Result<Json<PruneMissingResponse>, AppError> {
    let pruned_count = prune_missing_items(
        &context.pool,
        &context.settings.ingest,
        user.id,
        payload.ids.as_deref(),
    )
    .await?;
    Ok(Json(PruneMissingResponse { pruned_count }))
}
