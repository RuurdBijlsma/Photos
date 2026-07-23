use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::extract::State;
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::trash::interfaces::TrashBatchRequest;
use common_services::api::trash::service::{
    get_trash_items, perma_delete_items, restore_items, soft_delete_items,
};
use common_types::pb::api::OrderedMediaResponse;
use sqlx::PgPool;
use tracing::instrument;

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_trash_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Protobuf<OrderedMediaResponse>, AppError> {
    let timeline = get_trash_items(&pool, user.id).await?;
    Ok(Protobuf(timeline))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn soft_delete_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<TrashBatchRequest>,
) -> Result<(), AppError> {
    soft_delete_items(&pool, user.id, &payload.ids).await?;
    Ok(())
}

#[instrument(skip(context, user), err(Debug))]
pub async fn perma_delete_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<TrashBatchRequest>,
) -> Result<(), AppError> {
    let media_root = &context.settings.ingest.media_root;
    let thumbnail_root = &context.settings.ingest.thumbnails_root;

    perma_delete_items(
        &context.pool,
        user.id,
        &payload.ids,
        media_root,
        thumbnail_root,
    )
    .await?;

    Ok(())
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn restore_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Json(payload): Json<TrashBatchRequest>,
) -> Result<(), AppError> {
    restore_items(&pool, user.id, &payload.ids).await?;
    Ok(())
}
