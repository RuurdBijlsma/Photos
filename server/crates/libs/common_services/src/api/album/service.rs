use super::interfaces::{AcceptInviteRequest, AlbumShareClaims, AlbumSort, SharedMediaItem};
use crate::api::app_error::AppError;
use crate::database::album::album::{Album, AlbumRole, AlbumSummary};
use crate::database::album::album_collaborator::AlbumCollaborator;
use crate::database::album_store::AlbumStore;
use crate::database::jobs::JobType;
use crate::database::key_vector_store::KeyVectorStore;
use crate::database::media_item_store::MediaItemStore;
use crate::database::user_store::UserStore;
use crate::database::{CreateAlbumPayload, UpdateField};
use crate::job_queue::enqueue_job;
use crate::s2s_client::{S2SClient, insecure_extract_token_claims};
use crate::utils::nice_id;
use app_state::{IngestSettings, constants};
use chrono::{Duration, Utc};
use color_eyre::eyre::Context;
use common_types::ImportAlbumItemPayload;
use common_types::pb::api::{AlbumInfo, FullAlbumMediaResponse, SimpleTimelineItem};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::{Executor, PgPool, PgTransaction, Postgres};
use tracing::instrument;

const DEFAULT_ALBUM_SORT: AlbumSort = AlbumSort::DateAsc;

#[instrument(skip(executor))]
async fn check_user_role(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: i32,
    album_id: &str,
    required_roles: &[AlbumRole],
) -> Result<bool, AppError> {
    let role = AlbumStore::find_user_role(executor, album_id, user_id).await?;

    match role {
        Some(r) if required_roles.contains(&r) => Ok(true),
        _ => Ok(false),
    }
}

async fn can_edit_album(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: i32,
    album_id: &str,
) -> Result<bool, AppError> {
    check_user_role(
        executor,
        user_id,
        album_id,
        &[AlbumRole::Owner, AlbumRole::Contributor],
    )
    .await
}

async fn can_view_album(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: i32,
    album_id: &str,
) -> Result<bool, AppError> {
    check_user_role(
        executor,
        user_id,
        album_id,
        &[AlbumRole::Owner, AlbumRole::Contributor, AlbumRole::Viewer],
    )
    .await
}

#[instrument(skip(executor))]
async fn is_album_owner<'c, E>(executor: E, user_id: i32, album_id: &str) -> Result<bool, AppError>
where
    E: Executor<'c, Database = Postgres>,
{
    check_user_role(executor, user_id, album_id, &[AlbumRole::Owner]).await
}

/// Finds a representative thumbnail for a list of media items.
/// Returns the image that has an embedding closest to the centroid if >50% have embeddings.
/// Otherwise, returns the middle item chronologically.
#[instrument(skip(tx))]
pub async fn get_representative_thumbnail(
    tx: &mut PgTransaction<'_>,
    media_item_ids: &[String],
) -> Result<Option<String>, AppError> {
    if media_item_ids.is_empty() {
        return Ok(None);
    }

    // Try contrastive logic if more than 50% have embeddings
    let cached_global_centroid = KeyVectorStore::get_vector(&mut **tx, "global_centroid").await?;

    let result = sqlx::query!(
        r#"
        WITH target_embeddings AS (
            SELECT media_item_id, embedding
            FROM visual_analysis
            WHERE media_item_id = ANY($1)
            AND deleted = false
        ),
        stats AS (
            SELECT COUNT(*) as count FROM target_embeddings
        ),
        album_centroid AS (
            SELECT avg(embedding)::vector as center FROM target_embeddings
        ),
        global_centroid_val AS (
            SELECT
                CASE
                    WHEN $2::vector IS NOT NULL THEN $2::vector
                    ELSE (
                        SELECT avg(embedding)::vector
                        FROM (
                            SELECT embedding
                            FROM visual_analysis
                            WHERE deleted = false
                            ORDER BY random()
                            LIMIT 1000
                        ) as sample
                    )
                END as center
        ),
        scored_items AS (
            SELECT
                t.media_item_id,
                -- Score = (Distance to Global) - (Distance to Album)
                (t.embedding <=> gc.center) - (t.embedding <=> ac.center) as score
            FROM target_embeddings t, album_centroid ac, global_centroid_val gc
            WHERE (SELECT count FROM stats) > (array_length($1, 1) / 2)
        )
        SELECT
            (SELECT count FROM stats)::bigint as "embedded_count!",
            (SELECT media_item_id FROM scored_items ORDER BY score DESC LIMIT 1)::text as "closest_id?"
        "#,
        media_item_ids,
        cached_global_centroid.as_deref() as Option<&[f32]>
    )
    .fetch_one(&mut **tx)
    .await?;

    if let Some(id) = result.closest_id {
        return Ok(Some(id));
    }

    // Fallback: Middle item chronologically
    let chronological_ids = sqlx::query_scalar!(
        "SELECT id FROM media_item WHERE id = ANY($1) AND deleted = false ORDER BY sort_timestamp",
        media_item_ids
    )
    .fetch_all(&mut **tx)
    .await?;

    if chronological_ids.is_empty() {
        return Ok(None);
    }

    let middle_idx = chronological_ids.len() / 2;
    Ok(chronological_ids.get(middle_idx).cloned())
}

#[instrument(skip(pool))]
pub async fn create_album(
    pool: &PgPool,
    user_id: i32,
    name: &str,
    description: Option<String>,
    is_public: bool,
    media_item_ids: &[String],
) -> Result<Album, AppError> {
    let mut tx = pool.begin().await?;
    let album_id = nice_id(constants().database.media_item_id_length);

    let album = AlbumStore::create(
        &mut *tx,
        &album_id,
        CreateAlbumPayload {
            owner_id: user_id,
            name: name.to_owned(),
            description,
            thumbnail_id: None,
            sort_mode: DEFAULT_ALBUM_SORT,
            is_public,
        },
    )
    .await?;
    AlbumStore::upsert_collaborator(&mut *tx, &album.id, user_id, AlbumRole::Owner).await?;
    if !media_item_ids.is_empty() {
        AlbumStore::add_media_items(&mut *tx, &album_id, media_item_ids, user_id).await?;
        if let Some(thumb) = get_representative_thumbnail(&mut tx, media_item_ids).await? {
            AlbumStore::update(
                &mut *tx,
                &album_id,
                None,
                UpdateField::Ignore,
                UpdateField::Value(thumb),
                None,
            )
            .await?;
        }
    }

    AlbumStore::sort_media_items(&mut tx, &album_id, DEFAULT_ALBUM_SORT).await?;

    tx.commit().await?;

    Ok(album)
}

/// Adds one or more media items to an album.
/// The user must be an owner or contributor.
#[instrument(skip(pool))]
pub async fn add_media_to_album(
    pool: &PgPool,
    album_id: &str,
    media_item_ids: &[String],
    user_id: i32,
) -> Result<(), AppError> {
    if !can_edit_album(pool, user_id, album_id).await? {
        return Err(AppError::NotFound("Album not found.".to_string()));
    }

    let mut tx = pool.begin().await?;

    // Get current album state
    let Some(album) = AlbumStore::find_by_id(&mut *tx, album_id).await? else {
        return Err(AppError::NotFound("Album not found.".to_string()));
    };

    // Add the items
    AlbumStore::add_media_items(&mut *tx, album_id, media_item_ids, user_id).await?;

    // If NOT manually sorted, run the reorder logic immediately to interleave them
    if album.sort_mode != AlbumSort::None {
        AlbumStore::sort_media_items(&mut tx, album_id, album.sort_mode).await?;
    }

    if album.thumbnail_id.is_none()
        && let Some(tid) = get_representative_thumbnail(&mut tx, media_item_ids).await?
    {
        AlbumStore::update(
            &mut *tx,
            album_id,
            None,
            UpdateField::Ignore,
            UpdateField::Value(tid),
            None,
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Removes a media item from an album.
/// The user must be an owner or contributor.
#[instrument(skip(pool))]
pub async fn remove_media_from_album(
    pool: &PgPool,
    album_id: &str,
    media_item_ids: &[String],
    user_id: i32,
) -> Result<(), AppError> {
    if !can_edit_album(pool, user_id, album_id).await? {
        return Err(AppError::NotFound(
            "Album not found or permission denied.".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;
    AlbumStore::remove_media_items_by_id(&mut *tx, album_id, media_item_ids).await?;

    // Fix thumbnail id if it was removed
    if let Some(album) = AlbumStore::find_by_id(&mut *tx, album_id).await? {
        // Check if removed item was the thumbnail
        for media_item_id in media_item_ids {
            if Some(media_item_id) == album.thumbnail_id.as_ref() && album.media_count > 0 {
                let ids: Vec<String> = sqlx::query_scalar!(
                    "SELECT media_item_id FROM album_media_item WHERE album_id = $1",
                    album_id
                )
                .fetch_all(&mut *tx)
                .await?;

                if let Some(thumbnail_id) = get_representative_thumbnail(&mut tx, &ids).await? {
                    AlbumStore::update(
                        &mut *tx,
                        album_id,
                        None,
                        UpdateField::Ignore,
                        UpdateField::Value(thumbnail_id),
                        None,
                    )
                    .await?;
                }
                break;
            }
        }
    }
    tx.commit().await?;

    Ok(())
}

/// Adds a new user as a collaborator to an album.
/// The inviting user must be the album owner.
#[instrument(skip(pool))]
pub async fn add_collaborator(
    pool: &PgPool,
    album_id: &str,
    new_user_id: i32,
    role: AlbumRole,
    inviting_user_id: i32,
) -> Result<AlbumCollaborator, AppError> {
    // The owner is the only one who can add collaborators.
    if !is_album_owner(pool, inviting_user_id, album_id).await? {
        return Err(AppError::NotFound(
            "Album not found or permission denied.".to_string(),
        ));
    }

    // Find the user to add by their email.
    let user_to_add = UserStore::find_by_id(pool, new_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User with id {new_user_id} not found.")))?;

    // An owner cannot be added or demoted via this function.
    if role == AlbumRole::Owner {
        return Err(AppError::Internal(color_eyre::eyre::eyre!(
            "Cannot assign the owner role."
        )));
    }

    // Insert the new collaborator, or update their role if they already exist.
    let new_collaborator =
        AlbumStore::upsert_collaborator(pool, album_id, user_to_add.id, role).await?;

    Ok(new_collaborator)
}

/// Removes a collaborator from an album.
/// The user performing the action must be (the album owner, or the collaborator to be removed).
#[instrument(skip(pool))]
pub async fn remove_collaborator(
    pool: &PgPool,
    album_id: &str,
    collaborator_id_to_remove: i64,
    requesting_user_id: i32,
) -> Result<(), AppError> {
    let Some(collaborator_to_remove) =
        AlbumStore::find_collaborator_by_id(pool, collaborator_id_to_remove).await?
    else {
        return Err(AppError::NotFound("Collaborator not found.".to_string()));
    };
    let is_self_removal = collaborator_to_remove.user_id == requesting_user_id;
    let is_owner = is_album_owner(pool, requesting_user_id, album_id).await?;
    if !is_owner && !is_self_removal {
        return Err(AppError::NotFound(
            "Album not found or permission denied.".to_string(),
        ));
    }

    // The owner cannot be removed.
    if matches!(collaborator_to_remove.role, AlbumRole::Owner) {
        return Err(AppError::Internal(color_eyre::eyre::eyre!(
            "The album owner cannot be removed."
        )));
    }

    AlbumStore::remove_collaborator_by_id(pool, collaborator_id_to_remove).await?;

    Ok(())
}

pub async fn delete_album(pool: &PgPool, album_id: &str, user_id: i32) -> Result<(), AppError> {
    // Permission Check: Only the owner can update album details.
    if !is_album_owner(pool, user_id, album_id).await? {
        return Err(AppError::NotFound(
            "Album not found or permission denied.".to_string(),
        ));
    }
    AlbumStore::delete(pool, album_id, user_id).await?;
    Ok(())
}

/// Updates an album's name and/or description.
/// The user must be the album owner.
#[instrument(skip(pool))]
pub async fn update_album(
    pool: &PgPool,
    album_id: &str,
    user_id: i32,
    name: Option<String>,
    description: UpdateField<String>,
    thumbnail_id: UpdateField<String>,
    is_public: Option<bool>,
) -> Result<Album, AppError> {
    // Permission Check: Only the owner can update album details.
    if !is_album_owner(pool, user_id, album_id).await? {
        return Err(AppError::NotFound(
            "Album not found or permission denied.".to_string(),
        ));
    }

    // At least one field must be provided for the update.
    if name.is_none() && description.is_ignore() && thumbnail_id.is_ignore() && is_public.is_none()
    {
        let album = AlbumStore::find_by_id(pool, album_id)
            .await?
            .ok_or_else(|| AppError::NotFound(album_id.to_owned()))?;
        return Ok(album);
    }

    if let UpdateField::Value(thumbnail_id) = &thumbnail_id {
        let exists = AlbumStore::has_media_item(pool, album_id, thumbnail_id).await?;
        if !exists {
            return Err(AppError::BadRequest(
                "thumbnail_id is not in the album".to_owned(),
            ));
        }
    }

    let updated_album =
        AlbumStore::update(pool, album_id, name, description, thumbnail_id, is_public).await?;
    Ok(updated_album)
}

#[instrument(skip(pool))]
pub async fn generate_invite(
    pool: &PgPool,
    public_url: String,
    jwt_secret: String,
    album_id: &str,
    user_id: i32,
    user_name: &str,
) -> Result<String, AppError> {
    // Permission Check: Only the owner can generate an invite.
    if !is_album_owner(pool, user_id, album_id).await? {
        return Err(AppError::Forbidden(
            "Only the album owner can generate an invitation.".to_string(),
        ));
    }

    let expires_at = (Utc::now()
        + Duration::minutes(constants().auth.album_invitation_expiry_minutes))
    .timestamp();

    let claims = AlbumShareClaims {
        iss: public_url.clone(),
        sub: album_id.to_owned(),
        exp: expires_at,
        sharer_username: user_name.to_owned(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )?;

    Ok(token)
}

/// Accepts an album invitation and enqueues a background job to start the import.
pub async fn accept_invite(
    pool: &PgPool,
    settings: &IngestSettings,
    s2s_client: &S2SClient,
    user_id: i32,
    payload: AcceptInviteRequest,
) -> Result<Album, AppError> {
    let claims = insecure_extract_token_claims(&payload.token)
        .map_err(|_| AppError::Forbidden("Invalid token.".to_string()))?;

    let summary: AlbumSummary = s2s_client
        .get_album_invite_summary(&payload.token)
        .await
        .wrap_err("Failed to get album invite summary from remote server")?;

    // 2. Create the new album locally
    let album_id = nice_id(constants().database.album_id_length);
    let mut tx = pool.begin().await?;
    let description = payload.description.filter(|d| !d.trim().is_empty());

    let album = AlbumStore::create(
        &mut *tx,
        &album_id,
        CreateAlbumPayload {
            owner_id: user_id,
            name: payload.name,
            description,
            thumbnail_id: None,
            sort_mode: DEFAULT_ALBUM_SORT,
            is_public: false,
        },
    )
    .await?;
    AlbumStore::upsert_collaborator(&mut *tx, &album_id, user_id, AlbumRole::Owner).await?;
    tx.commit().await?;

    // 3. For each media item, enqueue a download & import job
    for relative_path in summary.relative_paths {
        let item_payload = ImportAlbumItemPayload {
            remote_relative_path: relative_path,
            local_album_id: album_id.clone(),
            remote_username: claims.sharer_username.clone(),
            remote_url: claims.iss.parse()?,
            token: payload.token.clone(),
        };

        enqueue_job(pool, settings, JobType::ImportAlbumItem)
            .user_id(user_id)
            .payload(&item_payload)
            .call()
            .await?;
    }

    Ok(album)
}

// ====================================== //
// === --- === ALBUM TIMELINE === --- === //
// ====================================== //

/// Fetches media items for an album by specific Rank Groups.
/// Used to hydrate the grid as the user scrolls.
#[instrument(skip(pool))]
pub async fn get_album_media(
    pool: &PgPool,
    album_id: &str,
    user_id: Option<i32>,
) -> Result<FullAlbumMediaResponse, AppError> {
    let Some(album) = AlbumStore::find_by_id(pool, album_id).await? else {
        return Err(AppError::NotFound(album_id.to_owned()));
    };
    if !album.is_public {
        let Some(uid) = user_id else {
            return Err(AppError::NotFound(album_id.to_string()));
        };
        if !can_view_album(pool, uid, album_id).await? {
            return Err(AppError::NotFound(album_id.to_string()));
        }
    }
    let items_future = AlbumStore::list_sorted_media_items(pool, album_id, AlbumSort::None);
    let collaborators_future = AlbumStore::list_collaborators(pool, album_id);
    let (items, collaborators) = tokio::try_join!(items_future, collaborators_future)?;

    Ok(FullAlbumMediaResponse {
        items,
        album: Some(AlbumInfo {
            id: album.id,
            name: album.name,
            description: album.description,
            is_public: album.is_public,
            owner_id: album.owner_id,
            thumbnail_id: album.thumbnail_id,
            created_at: album.created_at.to_rfc3339(),
            first_date: album.earliest_media_item_timestamp.map(|d| d.to_string()),
            last_date: album.latest_media_item_timestamp.map(|d| d.to_string()),
            sort_mode: format!("{:?}", album.sort_mode),
            collaborators,
        }),
    })
}

pub async fn get_album_media_item(
    pool: &PgPool,
    album_id: &str,
    media_item_id: &str,
) -> Result<SharedMediaItem, AppError> {
    let Some(album) = AlbumStore::find_by_id(pool, album_id).await? else {
        return Err(AppError::NotFound(album_id.to_owned()));
    };
    if !album.is_public {
        return Err(AppError::NotFound(album_id.to_string()));
    }
    let has = AlbumStore::has_media_item(pool, album_id, media_item_id).await?;
    if !has {
        return Err(AppError::NotFound(album_id.to_string()));
    }
    let Some(item) = MediaItemStore::find_by_id(pool, media_item_id).await? else {
        return Err(AppError::NotFound(album_id.to_string()));
    };
    Ok(item.into())
}

// Re-ordering album logic

#[instrument(skip(pool))]
pub async fn get_sorted_album_media(
    pool: &PgPool,
    album_id: &str,
    user_id: i32,
    sort_mode: AlbumSort,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    if !can_view_album(pool, user_id, album_id).await? {
        return Err(AppError::Forbidden("Permission denied".into()));
    }

    Ok(AlbumStore::list_sorted_media_items(pool, album_id, sort_mode).await?)
}

#[instrument(skip(pool))]
pub async fn move_album_item(
    pool: &PgPool,
    album_id: &str,
    user_id: i32,
    media_item_id: &str,
    new_rank: f64,
) -> Result<(), AppError> {
    if !can_edit_album(pool, user_id, album_id).await? {
        return Err(AppError::Forbidden("Permission denied".into()));
    }

    let mut tx = pool.begin().await?;

    // Update the rank
    sqlx::query!(
        "UPDATE album_media_item SET rank = $1 WHERE album_id = $2 AND media_item_id = $3",
        new_rank,
        album_id,
        media_item_id
    )
    .execute(&mut *tx)
    .await?;

    // Mark album as manually sorted
    sqlx::query!(
        "UPDATE album SET sort_mode = 'date_asc' WHERE id = $1",
        album_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[instrument(skip(pool))]
pub async fn reorder_media_items(
    pool: &PgPool,
    album_id: &str,
    user_id: i32,
    media_item_ids: &[String],
    sort_mode: AlbumSort,
) -> Result<(), AppError> {
    if !can_edit_album(pool, user_id, album_id).await? {
        return Err(AppError::Forbidden("Permission denied".into()));
    }

    let mut tx = pool.begin().await?;

    AlbumStore::reorder_media_items(&mut tx, album_id, media_item_ids).await?;

    // Mark album as manually sorted
    sqlx::query!(
        "UPDATE album SET sort_mode = $2 WHERE id = $1",
        album_id,
        sort_mode as AlbumSort
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
