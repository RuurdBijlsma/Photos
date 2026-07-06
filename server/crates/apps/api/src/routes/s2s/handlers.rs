use crate::api_state::ApiContext;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use common_services::api::app_error::AppError;
use common_services::api::s2s::interfaces::DownloadParams;
use common_services::api::s2s::service::{
    get_invite_summary, get_media_item_path, validate_token_for_media_item,
};
use common_services::database::album::album::AlbumSummary;
use common_services::database::media_item_store::MediaItemStore;
use tokio_util::io::ReaderStream;
use tracing::instrument;

#[instrument(skip(context), err(Debug))]
pub async fn invite_summary_handler(
    State(context): State<ApiContext>,
    TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<AlbumSummary>, AppError> {
    let token = authorization.token();
    let summary = get_invite_summary(&context.pool, token, &context.settings.secrets.jwt).await?;
    Ok(Json(summary))
}

#[instrument(skip(context), err(Debug))]
pub async fn download_file_handler(
    State(context): State<ApiContext>,
    Query(query): Query<DownloadParams>,
    TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let token = authorization.token();
    let Some(media_item_id) =
        MediaItemStore::find_id_by_relative_path(&context.pool, &query.relative_path).await?
    else {
        return Err(AppError::NotFound("File does not exist in db".to_owned()));
    };
    validate_token_for_media_item(
        &context.pool,
        token,
        &context.settings.secrets.jwt,
        &media_item_id,
    )
    .await?;
    let file_path = get_media_item_path(
        &context.pool,
        &context.settings.ingest.media_root,
        &media_item_id,
    )
    .await?;
    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Guess the MIME type from the file path.
    let mime_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .as_ref()
        .to_string();

    let file = tokio::fs::File::open(&file_path).await.map_err(|_| {
        AppError::NotFound(format!(
            "File not found on disk for item {}",
            &media_item_id
        ))
    })?;

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    // Use the dynamically determined MIME type in the response header.
    let headers = [
        (header::CONTENT_TYPE, mime_type),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        ),
    ];

    Ok((headers, body))
}
