use super::interfaces::{CommitImportRequest, ExchangeCodeRequest, GooglePhotosConfig, GooglePhotosConfigStatus, GooglePhotosUserStatus, MatchedItem, PickedMediaItem, PickedMediaItemsResponse, PickerSession, PollResult, StartPickerResponse, TokenResponse, UnmatchedItem};
use crate::api::album::interfaces::AlbumSort;
use crate::api::album::service::create_album;
use crate::api::app_error::AppError;
use crate::database::CreateAlbumPayload;
use crate::database::album::album::{Album, AlbumRole};
use crate::database::album_store::AlbumStore;
use crate::database::key_json_store::KeyJsonStore;
use crate::utils::nice_id;
use app_state::constants;
use chrono::Utc;
use color_eyre::eyre::eyre;
use sqlx::PgPool;
use tracing::instrument;

const GOOGLE_PHOTOS_CONFIG_KEY: &str = "google_photos_config";
const PICKER_API_BASE: &str = "https://photospicker.googleapis.com/v1";
const DEFAULT_ALBUM_SORT: AlbumSort = AlbumSort::DateAsc;

// ─── Admin: config management ────────────────────────────────────────────────

/// Load the stored Google Photos OAuth config. Returns `None` if not yet configured.
pub async fn get_google_config(pool: &PgPool) -> Result<Option<GooglePhotosConfig>, AppError> {
    let value = KeyJsonStore::get_value(pool, GOOGLE_PHOTOS_CONFIG_KEY, None).await?;
    match value {
        None => Ok(None),
        Some(v) => {
            let config: GooglePhotosConfig =
                serde_json::from_value(v).map_err(|e| AppError::Internal(eyre!(e)))?;
            Ok(Some(config))
        }
    }
}

/// Persist Google Photos OAuth credentials (admin only).
pub async fn save_google_config(pool: &PgPool, config: GooglePhotosConfig) -> Result<(), AppError> {
    let value = serde_json::to_value(&config).map_err(|e| AppError::Internal(eyre!(e)))?;
    KeyJsonStore::set_value(pool, GOOGLE_PHOTOS_CONFIG_KEY, &value, None).await?;
    Ok(())
}

/// Return config status for admin UI (does not expose the secret).
pub async fn get_google_config_status(pool: &PgPool) -> Result<GooglePhotosConfigStatus, AppError> {
    let config = get_google_config(pool).await?;
    Ok(match config {
        None => GooglePhotosConfigStatus {
            configured: false,
            client_id_preview: None,
        },
        Some(c) => {
            let preview = c.client_id.chars().take(8).collect::<String>();
            GooglePhotosConfigStatus {
                configured: true,
                client_id_preview: Some(format!("{preview}...")),
            }
        }
    })
}

// ─── OAuth code exchange ──────────────────────────────────────────────────────

/// Exchange a Google OAuth authorization code for an access token.
pub async fn exchange_code_for_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, AppError> {
    let params = [
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(eyre!(e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Google token exchange failed ({status}): {body}"
        )));
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(eyre!(e)))?;
    Ok(token_resp.access_token)
}

// ─── Picker session management ────────────────────────────────────────────────

/// Create a Picker session with Google's API and return the session details.
async fn create_picker_session(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<PickerSession, AppError> {
    let resp = client
        .post(format!("{PICKER_API_BASE}/sessions"))
        .bearer_auth(access_token)
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| AppError::Internal(eyre!(e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Google Picker session creation failed ({status}): {body}"
        )));
    }

    resp.json::<PickerSession>()
        .await
        .map_err(|e| AppError::Internal(eyre!(e)))
}

/// Persist an import session to the database and return its generated ID.
async fn insert_import_session(
    pool: &PgPool,
    user_id: i32,
    picker_session_id: &str,
    picker_uri: &str,
    access_token: &str,
) -> Result<String, AppError> {
    let session_id = nice_id(16);
    let expires_at = Utc::now() + chrono::Duration::hours(1);

    sqlx::query!(
        r#"
        INSERT INTO google_photos_import_session
            (id, user_id, picker_session_id, picker_uri, access_token, status, expires_at)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6)
        "#,
        session_id,
        user_id,
        picker_session_id,
        picker_uri,
        access_token,
        expires_at,
    )
    .execute(pool)
    .await?;

    Ok(session_id)
}

// ─── Public service functions ─────────────────────────────────────────────────

/// Full flow: exchange OAuth code → create Picker session → store in DB → return IDs.
#[instrument(skip(pool, payload))]
pub async fn start_picker(
    pool: &PgPool,
    user_id: i32,
    payload: ExchangeCodeRequest,
) -> Result<StartPickerResponse, AppError> {
    let config = get_google_config(pool).await?.ok_or_else(|| {
        AppError::BadRequest(
            "Google Photos integration is not configured. Ask your admin to set it up.".to_string(),
        )
    })?;

    let client = reqwest::Client::new();

    let access_token = exchange_code_for_token(
        &client,
        &config.client_id,
        &config.client_secret,
        &payload.code,
        &payload.redirect_uri,
    )
    .await?;

    let picker_session = create_picker_session(&client, &access_token).await?;

    let import_session_id = insert_import_session(
        pool,
        user_id,
        &picker_session.id,
        &picker_session.picker_uri,
        &access_token,
    )
    .await?;

    Ok(StartPickerResponse {
        import_session_id,
        picker_uri: picker_session.picker_uri,
    })
}

struct ImportSession {
    picker_session_id: String,
    access_token: String,
    status: String,
}

/// Poll the Google Picker session. If done, fetch selected items and match them
/// against media_items already in the user's local library (by filename).
#[instrument(skip(pool))]
pub async fn poll_import_session(
    pool: &PgPool,
    import_session_id: &str,
    user_id: i32,
) -> Result<PollResult, AppError> {
    // Load session from DB
    let row = sqlx::query_as!(
        ImportSession,
        r#"
        SELECT picker_session_id, access_token, status
        FROM google_photos_import_session
        WHERE id = $1 AND user_id = $2 AND expires_at > now()
        "#,
        import_session_id,
        user_id,
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Import session not found or expired.".to_string()))?;

    // If already committed, return empty
    if row.status == "committed" {
        return Ok(PollResult {
            ready: true,
            matched: vec![],
            unmatched: vec![],
        });
    }

    let client = reqwest::Client::new();

    // Check with Google if user has finished selecting
    let session_resp = client
        .get(format!(
            "{PICKER_API_BASE}/sessions/{}",
            row.picker_session_id
        ))
        .bearer_auth(&row.access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(eyre!(e)))?;

    if !session_resp.status().is_success() {
        let status = session_resp.status();
        let body = session_resp.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "Google Picker session poll failed ({status}): {body}"
        )));
    }

    let session: PickerSession = session_resp
        .json()
        .await
        .map_err(|e| AppError::Internal(eyre!(e)))?;

    if session.media_items_set != Some(true) {
        // User hasn't finished selecting yet
        return Ok(PollResult {
            ready: false,
            matched: vec![],
            unmatched: vec![],
        });
    }

    // Paginate through all picked items
    let picked_items =
        fetch_all_picked_items(&client, &row.picker_session_id, &row.access_token).await?;

    // Match filenames against local library
    let filenames: Vec<String> = picked_items
        .iter()
        .filter_map(|item| item.media_file.as_ref().map(|f| f.filename.clone()))
        .collect();

    let matched_rows = sqlx::query!(
        r#"
        SELECT id, filename
        FROM media_item
        WHERE user_id = $1
          AND deleted = false
          AND filename = ANY($2::text[])
        "#,
        user_id,
        &filenames,
    )
    .fetch_all(pool)
    .await?;

    // Build a map: filename -> media_item_id (take first match per filename)
    use std::collections::HashMap;
    let mut filename_to_id: HashMap<String, String> = HashMap::new();
    for row in matched_rows {
        filename_to_id.entry(row.filename).or_insert(row.id);
    }

    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for item in &picked_items {
        let Some(media_file) = &item.media_file else {
            continue;
        };
        match filename_to_id.get(&media_file.filename) {
            Some(media_item_id) => {
                // Deduplicate: only add each media_item_id once
                if !matched
                    .iter()
                    .any(|m: &MatchedItem| m.media_item_id == *media_item_id)
                {
                    matched.push(MatchedItem {
                        media_item_id: media_item_id.clone(),
                        filename: media_file.filename.clone(),
                    });
                }
            }
            None => {
                unmatched.push(UnmatchedItem {
                    filename: media_file.filename.clone(),
                });
            }
        }
    }

    // Mark session as ready
    sqlx::query!(
        "UPDATE google_photos_import_session SET status = 'ready' WHERE id = $1",
        import_session_id,
    )
    .execute(pool)
    .await?;

    Ok(PollResult {
        ready: true,
        matched,
        unmatched,
    })
}

/// Paginate through all picked media items from the Google Picker API.
async fn fetch_all_picked_items(
    client: &reqwest::Client,
    picker_session_id: &str,
    access_token: &str,
) -> Result<Vec<PickedMediaItem>, AppError> {
    let mut all_items: Vec<PickedMediaItem> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut url =
            format!("{PICKER_API_BASE}/mediaItems?sessionId={picker_session_id}&pageSize=100");
        if let Some(token) = &page_token {
            url.push_str(&format!("&pageToken={token}"));
        }

        let resp = client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| AppError::Internal(eyre!(e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::BadRequest(format!(
                "Fetching picked media items failed ({status}): {body}"
            )));
        }

        let page: PickedMediaItemsResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(eyre!(e)))?;

        if let Some(items) = page.media_items {
            all_items.extend(items);
        }

        match page.next_page_token {
            Some(token) if !token.is_empty() => page_token = Some(token),
            _ => break,
        }
    }

    Ok(all_items)
}

/// Create a local album containing the confirmed media items, then mark session committed.
#[instrument(skip(pool))]
pub async fn commit_import(
    pool: &PgPool,
    user_id: i32,
    payload: CommitImportRequest,
) -> Result<Album, AppError> {
    // Verify session belongs to user and isn't already committed
    let row = sqlx::query!(
        r#"
        SELECT status
        FROM google_photos_import_session
        WHERE id = $1 AND user_id = $2 AND expires_at > now()
        "#,
        payload.import_session_id,
        user_id,
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Import session not found or expired.".to_string()))?;

    if row.status == "committed" {
        return Err(AppError::Conflict(
            "This import has already been committed.".to_string(),
        ));
    }

    if payload.album_name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Album name cannot be empty.".to_string(),
        ));
    }

    let description = payload.album_description.filter(|d| !d.trim().is_empty());

    let album = create_album(
        &pool,
        user_id,
        &payload.album_name,
        description,
        false,
        &payload.media_item_ids,
    )
    .await?;

    // Mark session as committed
    sqlx::query!(
        "UPDATE google_photos_import_session SET status = 'committed' WHERE id = $1",
        payload.import_session_id,
    )
    .execute(pool)
    .await?;

    Ok(album)
}

pub async fn get_google_user_status(
    pool: &PgPool,
) -> Result<GooglePhotosUserStatus, AppError> {
    let config = get_google_config(pool).await?;
    Ok(match config {
        None => GooglePhotosUserStatus {
            configured: false,
            client_id: None,
        },
        Some(c) => GooglePhotosUserStatus {
            configured: true,
            client_id: Some(c.client_id),
        },
    })
}