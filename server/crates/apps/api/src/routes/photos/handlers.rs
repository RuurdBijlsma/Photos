use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::album::interfaces::MediaItemWithAlbums;
use common_services::api::photos::interfaces::{
    DownloadMediaParams, GeoPhotosParams, UpdateMediaItemRequest,
};
use common_services::api::photos::service::{
    download_media_file, stream_video_file, thumbnail_on_demand_cached, update_media_item,
};
use common_services::database::album_store::AlbumStore;

use crate::api_state::ApiContext;
use crate::auth::middlewares::user::ApiUser;
use axum::http::header;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use axum_extra::headers::Range;
use common_services::api::app_error::AppError;
use common_services::api::photos::interfaces::PhotoThumbnailParams;
use common_services::database::cached_store::cached_store;
use common_services::database::media_item_store::MediaItemStore;
use common_types::pb::api::MapPhotosResponse;
use sqlx::PgPool;
use tracing::instrument;

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_full_item_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Path(media_item_id): Path<String>,
) -> Result<Json<MediaItemWithAlbums>, AppError> {
    let item = MediaItemStore::find_by_id(&pool, &media_item_id).await?;
    if let Some(item) = item
        && item.user_id == user.id
    {
        Ok(Json(MediaItemWithAlbums {
            media_item: item,
            albums: AlbumStore::list_for_media_item(&pool, user.id, &media_item_id).await?,
        }))
    } else {
        Err(AppError::NotFound(media_item_id))
    }
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn update_media_item_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Path(media_item_id): Path<String>,
    Json(payload): Json<UpdateMediaItemRequest>,
) -> Result<(), AppError> {
    update_media_item(&pool, &media_item_id, user.id, &payload).await?;

    Ok(())
}

pub async fn download_full_file_by_rel_path(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(query): Query<DownloadMediaParams>,
) -> Result<impl IntoResponse, AppError> {
    let user_media_folder = cached_store()
        .get_user_media_folder(&context.pool, user.id)
        .await?;
    let response = download_media_file(
        &context.settings.ingest,
        user.id,
        user.role,
        user_media_folder,
        &query.path,
    )
    .await?;
    Ok(response)
}

pub async fn download_full_file_by_id(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(media_item_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let store = cached_store();
    let item = store
        .get_rel_path_and_user_id_for_media_item(&context.pool, &media_item_id)
        .await?;
    let user_media_folder = store.get_user_media_folder(&context.pool, user.id).await?;
    let response = download_media_file(
        &context.settings.ingest,
        user.id,
        user.role,
        user_media_folder,
        &item.relative_path,
    )
    .await?;
    Ok(response)
}

#[instrument(skip(context), err(Debug))]
pub async fn get_photo_thumbnail(
    State(context): State<ApiContext>,
    Query(query): Query<PhotoThumbnailParams>,
    Path(media_item_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let size = query.size.unwrap_or(360);
    if size > 1440 {
        return Err(AppError::Forbidden("Denied".to_owned()));
    }

    let image_bytes = thumbnail_on_demand_cached(
        &context.pool,
        size,
        &media_item_id,
        &context.settings.ingest,
    )
    .await?;

    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        image_bytes,
    ))
}

pub async fn stream_video_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(media_item_id): Path<String>,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse, AppError> {
    let range_inner = range.map(|TypedHeader(r)| r);
    stream_video_file(
        &context.pool,
        &context.settings.ingest,
        user.id,
        user.role,
        &media_item_id,
        range_inner,
    )
    .await
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_geo_photos_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<GeoPhotosParams>,
) -> Result<Protobuf<MapPhotosResponse>, AppError> {
    let items =
        MediaItemStore::find_all_geo_by_user_id(&pool, user.id, params.start_date, params.end_date)
            .await?;
    Ok(Protobuf(MapPhotosResponse { items }))
}

#[instrument(skip(pool), err(Debug))]
pub async fn get_pano_config(
    State(pool): State<PgPool>,
    Path(media_item_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let Some(full_media_item) = MediaItemStore::find_by_id(&pool, &media_item_id).await? else {
        return Err(AppError::NotFound("Media Item not found".to_owned()));
    };
    let Some(pano_config) = full_media_item.panorama_config else {
        return Err(AppError::NotFound("Media Item not a pano".to_owned()));
    };
    Ok(Json(pano_config))
}
