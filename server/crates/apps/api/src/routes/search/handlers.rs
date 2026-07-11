use crate::api_state::ApiContext;
use axum::extract::{Multipart, Path, Query, State};
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use color_eyre::eyre;
use common_services::api::app_error::AppError;
use common_services::api::search::handler_utils::to_search_config;
use common_services::api::search::interfaces::{
    SearchFilterRanges, SearchImage, SearchParams, SearchSuggestionsParams, VisionQuery,
};
use common_services::api::search::service::{
    get_random_search_suggestion, get_search_suggestions, search_by_image, search_by_media_items,
    search_filter_ranges, search_media,
};
use common_types::pb::api::{SearchResponse, SearchSuggestionsResponse};
use image::ImageReader;
use std::io::Cursor;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;
use crate::auth::middlewares::user::ApiUser;

/// Get a timeline of all media ratios, grouped by month.
///
/// # Errors
///
/// Returns a `AppError` if the database query fails.
#[instrument(skip(context, user), err(Debug))]
pub async fn get_search_results(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<SearchParams>,
) -> Result<Protobuf<SearchResponse>, AppError> {
    let items = search_media(
        user.id,
        &context.pool,
        context.text_embedder,
        params.clone().query,
        to_search_config(&context.settings.ingest.analyzer.search, params),
    )
    .await?;
    Ok(Protobuf(SearchResponse {
        items,
        session_id: None,
    }))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_search_suggestions_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<SearchSuggestionsParams>,
) -> Result<Protobuf<SearchSuggestionsResponse>, AppError> {
    let result = get_search_suggestions(user.id, &pool, &params.query, params.limit).await?;
    Ok(Protobuf(result))
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_random_search_suggestion_handler(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<String, AppError> {
    let result = get_random_search_suggestion(user.id, &pool).await?;
    Ok(result.unwrap_or_default())
}

#[instrument(skip(pool, user), err(Debug))]
pub async fn get_search_filter_ranges(
    State(pool): State<PgPool>,
    Extension(user): Extension<ApiUser>,
) -> Result<Json<SearchFilterRanges>, AppError> {
    let result = search_filter_ranges(user.id, &pool).await?;
    Ok(Json(result))
}

#[instrument(skip(context, user, multipart), err(Debug))]
pub async fn get_search_by_image_results(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Query(params): Query<SearchParams>,
    mut multipart: Multipart,
) -> Result<Protobuf<SearchResponse>, AppError> {
    const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB
    const ALLOWED_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "heic"];

    let mut image_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Internal(eyre::eyre!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("");
        if name == "image" {
            let file_name = field.file_name().unwrap_or("").to_string();
            let extension = file_name.rsplit('.').next().unwrap_or("").to_lowercase();

            if !ALLOWED_EXTENSIONS.contains(&extension.as_str()) {
                return Err(AppError::Internal(eyre::eyre!(
                    "Invalid image format. Allowed formats: {}",
                    ALLOWED_EXTENSIONS.join(", ")
                )));
            }

            // TODO: sometimes it can't read bytes for perfectly normal request
            // Image too big? idk
            let bytes = field.bytes().await.map_err(|e| {
                AppError::Internal(eyre::eyre!("Failed to read image bytes: {}", e))
            })?;

            if bytes.len() > MAX_IMAGE_SIZE {
                return Err(AppError::Internal(eyre::eyre!(
                    "Image too large. Maximum size is 10MB, got {}MB",
                    bytes.len() / (1024 * 1024)
                )));
            }

            image_bytes = Some(bytes.to_vec());
        }
    }

    let image_bytes = image_bytes
        .ok_or_else(|| AppError::Internal(eyre::eyre!("No image file provided in the request")))?;
    let img = ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()?
        .decode()?;
    let session_id = Uuid::new_v4();

    let items = search_by_image(
        user.id,
        &context.pool,
        context.text_embedder,
        context.vision_embedder,
        params.clone().query,
        VisionQuery::Raw(SearchImage {
            image: Some(img),
            session_id,
        }),
        to_search_config(&context.settings.ingest.analyzer.search, params),
        vec![],
    )
    .await?;
    Ok(Protobuf(SearchResponse {
        items,
        session_id: Some(session_id.to_string()),
    }))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_search_by_image_uuid(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<SearchParams>,
) -> Result<Protobuf<SearchResponse>, AppError> {
    let items = search_by_image(
        user.id,
        &context.pool,
        context.text_embedder,
        context.vision_embedder,
        params.clone().query,
        VisionQuery::Raw(SearchImage {
            image: None,
            session_id,
        }),
        to_search_config(&context.settings.ingest.analyzer.search, params),
        vec![],
    )
    .await?;
    Ok(Protobuf(SearchResponse {
        items,
        session_id: Some(session_id.to_string()),
    }))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_search_by_media_items(
    State(context): State<ApiContext>,
    Extension(user): Extension<ApiUser>,
    Path(media_item_ids): Path<String>,
    Query(params): Query<SearchParams>,
) -> Result<Protobuf<SearchResponse>, AppError> {
    let media_item_ids = media_item_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();
    let items = search_by_media_items(
        user.id,
        &context.pool,
        context.text_embedder,
        context.vision_embedder,
        params.clone().query,
        to_search_config(&context.settings.ingest.analyzer.search, params),
        &media_item_ids,
    )
    .await?;
    Ok(Protobuf(SearchResponse {
        items,
        session_id: None,
    }))
}
