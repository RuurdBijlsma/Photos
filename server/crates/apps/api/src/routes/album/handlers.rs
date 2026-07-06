use crate::api_state::ApiContext;
use crate::auth::middlewares::optional_user::OptionalUser;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use axum_extra::protobuf::Protobuf;
use common_services::api::album::backup_restore::{list_backups, restore_albums};

use app_state::IngestSettings;
use common_services::api::album::interfaces::{
    AcceptInviteRequest, AddCollaboratorRequest, AddMediaToAlbumRequest, BackupInfo,
    CheckInviteRequest, CreateAlbumRequest, GetSortedAlbumItemsRequest, ListAlbumsParam,
    ReorderMediaRequest, SharedMediaItem, UpdateAlbumRequest,
};
use common_services::api::album::service::{
    accept_invite, add_collaborator, add_media_to_album, create_album, delete_album,
    generate_invite, get_album_media, get_album_media_item, get_sorted_album_media,
    remove_collaborator, remove_media_from_album, reorder_media_items, update_album,
};
use common_services::api::app_error::AppError;
use common_services::database::album::album::{Album, AlbumSummary};
use common_services::database::album::album_collaborator::AlbumCollaborator;
use common_services::database::album_store::AlbumStore;
use common_services::database::app_user::User;
use common_types::pb::api::{FullAlbumMediaResponse, OrderedMediaResponse};
use tracing::instrument;

/// Create a new album.
///
/// The user creating the album will be designated as the owner.
#[instrument(skip(context, user, payload), err(Debug))]
pub async fn create_album_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Json(payload): Json<CreateAlbumRequest>,
) -> Result<(StatusCode, Json<Album>), AppError> {
    let album = create_album(
        &context.pool,
        user.id,
        &payload.name,
        payload.description,
        payload.is_public,
        &payload.media_item_ids,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(album)))
}

/// List all albums for the current user.
///
/// Returns all albums where the user is a collaborator (owner, contributor, or viewer).
#[instrument(skip(context, user), err(Debug))]
pub async fn get_user_albums_handler(
    State(context): State<ApiContext>,
    Query(query): Query<ListAlbumsParam>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<Album>>, AppError> {
    let albums = AlbumStore::list_with_count_by_user_id(
        &context.pool,
        user.id,
        query.sort_field,
        query.sort_direction,
    )
    .await?;
    Ok(Json(albums))
}

/// Update an album's details.
///
/// Allows updating the name and/or description. The user must be the album owner.
pub async fn update_album_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
    Json(payload): Json<UpdateAlbumRequest>,
) -> Result<Json<Album>, AppError> {
    let album = update_album(
        &context.pool,
        &album_id,
        user.id,
        payload.name,
        payload.description,
        payload.thumbnail_id,
        payload.is_public,
    )
    .await?;
    Ok(Json(album))
}

pub async fn delete_album_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
) -> Result<(), AppError> {
    delete_album(&context.pool, &album_id, user.id).await?;
    Ok(())
}

/// Add media items to an album.
///
/// The user must be an owner or contributor of the album.
#[instrument(skip(context, user), err(Debug))]
pub async fn add_media_to_album_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
    Json(payload): Json<AddMediaToAlbumRequest>,
) -> Result<StatusCode, AppError> {
    add_media_to_album(&context.pool, &album_id, &payload.media_item_ids, user.id).await?;
    Ok(StatusCode::OK)
}

/// Remove a media item from an album.
///
/// The user must be an owner or contributor of the album.
pub async fn remove_media_from_album_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path((album_id, media_item_ids)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let media_item_ids = media_item_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();
    remove_media_from_album(&context.pool, &album_id, &media_item_ids, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Add a collaborator to an album.
///
/// The inviting user must be the album owner.
pub async fn add_collaborator_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
    Json(payload): Json<AddCollaboratorRequest>,
) -> Result<Json<AlbumCollaborator>, AppError> {
    let collaborator = add_collaborator(
        &context.pool,
        &album_id,
        payload.user_id,
        payload.role,
        user.id,
    )
    .await?;
    Ok(Json(collaborator))
}

/// Remove a collaborator from an album.
///
/// The user performing the action must be the album owner. The owner cannot be removed.
pub async fn remove_collaborator_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path((album_id, user_id)): Path<(String, i64)>,
) -> Result<StatusCode, AppError> {
    remove_collaborator(&context.pool, &album_id, user_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(context, user), err(Debug))]
pub async fn get_sorted_album_items_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
    Query(request): Query<GetSortedAlbumItemsRequest>,
) -> Result<Protobuf<OrderedMediaResponse>, AppError> {
    let items =
        get_sorted_album_media(&context.pool, &album_id, user.id, request.sort_mode).await?;
    Ok(Protobuf(OrderedMediaResponse { items }))
}

#[instrument(skip(context, user), err(Debug))]
pub async fn reorder_media_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
    Json(payload): Json<ReorderMediaRequest>,
) -> Result<(), AppError> {
    reorder_media_items(
        &context.pool,
        &album_id,
        user.id,
        &payload.media_item_ids,
        payload.sort_mode,
    )
    .await?;
    Ok(())
}

/// Generate a cross-server invitation link for an album.
///
/// The inviting user must be the album owner. The generated token has a configurable expiry time.
pub async fn generate_invite_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(album_id): Path<String>,
) -> Result<Json<String>, AppError> {
    let token = generate_invite(
        &context.pool,
        context.settings.api.public_url,
        context.settings.secrets.jwt,
        &album_id,
        user.id,
        &user.name,
    )
    .await?;
    Ok(Json(token))
}

pub async fn check_invite_handler(
    State(context): State<ApiContext>,
    Json(payload): Json<CheckInviteRequest>,
) -> Result<Json<AlbumSummary>, AppError> {
    let summary = context
        .s2s_client
        .get_album_invite_summary(&payload.token)
        .await?;
    Ok(Json(summary))
}

/// Accept an album invitation.
///
/// This will enqueue a background job to begin the process of importing the album
/// from the remote server.
pub async fn accept_invite_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Json(payload): Json<AcceptInviteRequest>,
) -> Result<Json<Album>, AppError> {
    let album = accept_invite(
        &context.pool,
        &context.settings.ingest,
        &context.s2s_client,
        user.id,
        payload,
    )
    .await?;
    Ok(Json(album))
}

// ====================================== //
// === --- === ALBUM TIMELINE === --- === //
// ====================================== //

/// Get media items for specific rank groups within an album.
pub async fn get_album_media_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<OptionalUser>,
    Path(album_id): Path<String>,
) -> Result<Protobuf<FullAlbumMediaResponse>, AppError> {
    let response = get_album_media(&context.pool, &album_id, user.0.map(|u| u.id)).await?;
    Ok(Protobuf(response))
}

pub async fn get_album_media_item_handler(
    State(context): State<ApiContext>,
    Path((album_id, media_item_id)): Path<(String, String)>,
) -> Result<Json<SharedMediaItem>, AppError> {
    let response = get_album_media_item(&context.pool, &album_id, &media_item_id).await?;
    Ok(Json(response))
}

// ====================================== //
// === --- === BACKUP RESTORE === --- === //
// ====================================== //

pub async fn list_backups_handler(
    State(settings): State<IngestSettings>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<BackupInfo>>, AppError> {
    let backups = list_backups(&settings.cache_root, user.id).await?;
    Ok(Json(backups))
}

pub async fn restore_backup_handler(
    State(context): State<ApiContext>,
    Extension(user): Extension<User>,
    Path(backup_filename): Path<String>,
) -> Result<(), AppError> {
    let backup_path = context
        .settings
        .ingest
        .cache_root
        .join("albums")
        .join(user.id.to_string())
        .join(backup_filename);
    restore_albums(&context.pool, user.id, &backup_path).await?;
    Ok(())
}
