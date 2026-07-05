use crate::api_state::ApiContext;
use axum::extract::State;
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::app_error::AppError;
use common_services::api::trash::interfaces::TrashBatchRequest;
use common_services::api::trash::service::{
    get_trash_items, perma_delete_items, restore_items, soft_delete_items,
};
use common_services::database::app_user::User;
use common_types::pb::api::OrderedMediaResponse;
use tracing::instrument;

#[instrument(skip(context, user), err(Debug))]
pub async fn get_trash_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
) -> Result<Protobuf<OrderedMediaResponse>, AppError> {
    let timeline = get_trash_items(&context.pool, user.id).await?;
    Ok(Protobuf(timeline))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn soft_delete_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Json(payload): Json<TrashBatchRequest>,
) -> Result<(), AppError> {
    soft_delete_items(&context.pool, user.id, &payload.ids).await?;
    Ok(())
}

#[instrument(skip(context, user), err(Debug))]
pub async fn perma_delete_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
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

#[instrument(skip(context, user), err(Debug))]
pub async fn restore_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Json(payload): Json<TrashBatchRequest>,
) -> Result<(), AppError> {
    restore_items(&context.pool, user.id, &payload.ids).await?;
    Ok(())
}
