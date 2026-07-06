use crate::api_state::ApiContext;
use axum::Extension;
use axum::extract::State;
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::system::storage::{
    get_blurry_storage_items, get_large_storage_items, get_storage_summary,
};
use common_services::database::app_user::User;
use common_types::pb::api::{StorageReviewResponse, StorageSummaryResponse};
use tracing::instrument;

#[instrument(skip(context, user), err(Debug))]
pub async fn storage_summary_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Protobuf<StorageSummaryResponse>, AppError> {
    Ok(Protobuf(
        get_storage_summary(&context.pool, &context.settings.ingest, user.id).await?,
    ))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn storage_review_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Protobuf<StorageReviewResponse>, AppError> {
    Ok(Protobuf(
        get_large_storage_items(&context.pool, user.id).await?,
    ))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn storage_blurry_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Protobuf<StorageReviewResponse>, AppError> {
    Ok(Protobuf(
        get_blurry_storage_items(&context.pool, user.id).await?,
    ))
}
