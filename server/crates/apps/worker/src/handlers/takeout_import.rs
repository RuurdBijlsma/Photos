use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::require_media_mounted;
use app_state::IngestSettings;
use app_state::MakeRelativePath;
use app_state::constants::TAKEOUT_FOLDER;
use color_eyre::Result;
use color_eyre::eyre::{Context, eyre};
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::database::media_item::media_item::MediaItemIdAndFilename;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::takeout_link_store::{
    TakeoutLinkStore, attach_media_item_to_albums,
};
use common_services::database::user_store::UserStore;
use common_services::job_queue::bulk_enqueue_full_ingest_in_tx;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Enqueue payload for [`handle`], created by `handle_web_upload` when an
/// uploaded file turns out to be a Google Takeout zip.
#[derive(Debug, Serialize, Deserialize)]
pub struct TakeoutImportPayload {
    /// Zip location, relative to `app_data_root` (where `handle_web_upload` stages it).
    pub zip_relative_path: String,
    /// Batch id parsed from the zip filename (falls back to `"takeout"`).
    pub batch_id: String,
}

/// Advisory lock domain for takeout registration transactions. Serializes the
/// registration of concurrently arriving zips per user so that two zips
/// containing the same filename cannot both claim it.
const TAKEOUT_LOCK_OFFSET: i64 = 0x676F_6F64_0000_0000;

const ALBUM_METADATA_FILE: &str = "metadata.json";

#[derive(Debug, Default, Clone, Deserialize)]
struct TakeoutAlbumMetadata {
    title: Option<String>,
    description: Option<String>,
}

/// A media file extracted out of a takeout zip.
#[derive(Debug)]
struct ExtractedItem {
    /// First folder component under `Google Photos/` (`None` for root-level files).
    folder: Option<String>,
    /// Path of the file relative to that folder (may contain subcomponents if a
    /// takeout deviates from the flat layout).
    filename: String,
    /// Full path relative to `media_root` — the final `media_item.relative_path`.
    relative_path: String,
}

type AlbumMetadataMap = HashMap<String, TakeoutAlbumMetadata>;

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    if let Some(result) =
        require_media_mounted(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }
    let user_id = job
        .user_id
        .ok_or_else(|| eyre!("TakeoutImport job is missing a user_id"))?;
    let payload_value = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("TakeoutImport job is missing a payload"))?;
    let payload: TakeoutImportPayload = from_value(payload_value.clone())?;

    let ingest = &context.settings.ingest;
    let zip_path = ingest.app_data_root.join(&payload.zip_relative_path);
    if !fs::metadata(&zip_path).is_ok_and(|m| m.is_file()) {
        return Err(eyre!("Takeout zip file is missing: {}", zip_path.display()));
    }
    let user_media_folder = UserStore::get_user_media_folder(&context.pool, user_id)
        .await?
        .ok_or_else(|| eyre!("TakeoutImport: user {user_id} has no media folder set"))?;
    let takeout_root = ingest
        .media_root
        .join(&user_media_folder)
        .join(TAKEOUT_FOLDER);

    info!(
        "Importing takeout zip {:?} for user {user_id} (batch {})",
        zip_path, payload.batch_id
    );

    let (items, album_metadata) = extract_zip(&zip_path, &takeout_root, &ingest.media_root)?;
    debug!("Takeout zip album metadata: {:?}", album_metadata);

    let redundant_copies = register_extracted_items(
        &context.pool,
        ingest,
        user_id,
        &payload.batch_id,
        &items,
        &album_metadata,
    )
    .await?;

    // Registration is committed; redundant duplicate copies are safe to delete now.
    for path in &redundant_copies {
        if let Err(e) = fs::remove_file(path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!("Failed to delete redundant takeout copy {:?}: {}", path, e);
        }
    }

    // The zip itself is no longer needed; failure here must not fail the job.
    if let Err(e) = fs::remove_file(&zip_path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        warn!("Failed to delete takeout zip {:?}: {}", zip_path, e);
    }

    info!(
        "Takeout zip import finished: {} media file(s) registered, {} redundant copy/copies removed",
        items.len(),
        redundant_copies.len()
    );
    Ok(JobResult::Done)
}

/// Extracts a takeout zip into `destination_root`.
///
/// Only files somewhere under a `Google Photos/` folder are extracted. `*.json`
/// / `*.html` sidecars are skipped, except for the per-album `metadata.json`
/// which is read into memory (title + description). Extraction overwrites
/// existing files (idempotent re-runs).
fn extract_zip(
    zip_path: &Path,
    destination_root: &Path,
    media_root: &Path,
) -> color_eyre::Result<(Vec<ExtractedItem>, AlbumMetadataMap)> {
    let file = fs::File::open(zip_path)
        .wrap_err_with(|| format!("Opening takeout zip {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))?;

    let mut items = Vec::new();
    let mut album_metadata = AlbumMetadataMap::new();

    for index in 0..archive.len() {
        let mut entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Skipping unreadable zip entry {index}: {e}");
                continue;
            }
        };
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            warn!(
                "Skipping zip entry with unsafe path: {:?}",
                entry.name().unwrap_or_default()
            );
            continue;
        };
        let Some(relative) = photos_relative_path(&enclosed) else {
            debug!("Ignoring non-photos takeout entry: {:?}", enclosed);
            continue;
        };

        let (folder, file_path) = match relative.split_once('/') {
            Some((folder, rest)) => (Some(folder.to_owned()), rest),
            None => (None, relative.as_str()),
        };
        let file_name = file_path.rsplit('/').next().unwrap_or(file_path);
        let lower = file_path.to_lowercase();

        if [".json", ".html", ".csv", ".txt"]
            .iter()
            .any(|ext| lower.ends_with(ext))
        {
            if let Some(album_folder) =
                folder.filter(|_| file_name.eq_ignore_ascii_case(ALBUM_METADATA_FILE))
            {
                let mut buf = Vec::new();
                if entry.read_to_end(&mut buf).is_ok() {
                    match serde_json::from_slice::<TakeoutAlbumMetadata>(&buf) {
                        Ok(meta) => {
                            album_metadata.insert(album_folder, meta);
                        }
                        Err(e) => warn!("Unparseable album metadata.json: {e}"),
                    }
                }
            }
            continue;
        }

        if entry.size() == 0 {
            continue;
        }

        let destination = destination_root.join(&relative);
        if let Some(parent) = destination.parent()
            && let Err(e) = fs::create_dir_all(parent)
        {
            warn!("Failed to create takeout folder {:?}: {}", parent, e);
            continue;
        }
        let mut out = match fs::File::create(&destination) {
            Ok(out) => out,
            Err(e) => {
                warn!("Failed to create takeout file {:?}: {}", destination, e);
                continue;
            }
        };
        std::io::copy(&mut entry, &mut out)?;

        let relative_path = destination.make_relative(media_root)?;
        items.push(ExtractedItem {
            folder,
            filename: file_path.to_owned(),
            relative_path,
        });
    }

    Ok((items, album_metadata))
}

/// Reduces an entry path to the part below `Google Photos/`, stripping the
/// usual `Takeout/` wrapper. Returns `None` for other takeout products.
fn photos_relative_path(entry_path: &Path) -> Option<String> {
    let components: Vec<String> = entry_path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    let inner = if components.len() >= 2 && components[0].eq_ignore_ascii_case("Takeout") {
        &components[1..]
    } else {
        &components[..]
    };
    if inner.is_empty() {
        return None;
    }
    let relative = if inner[0].eq_ignore_ascii_case("Google Photos") {
        &inner[1..]
    } else {
        inner
    };
    if relative.is_empty() {
        None
    } else {
        Some(relative.join("/"))
    }
}

/// Decides, for every extracted file, whether it is the canonical copy of its
/// filename or a duplicate; registers albums, link rows and ingest jobs, all in
/// one transaction guarded by an advisory lock.
///
/// Returns the paths of redundant copies that should be deleted from disk.
async fn register_extracted_items(
    pool: &PgPool,
    ingest: &IngestSettings,
    user_id: i32,
    batch_id: &str,
    items: &[ExtractedItem],
    album_metadata: &AlbumMetadataMap,
) -> color_eyre::Result<Vec<PathBuf>> {
    if items.is_empty() {
        return Ok(Vec::new());
    }

    // Quick index of extract duplicates within this zip.
    let mut unique_items: Vec<&ExtractedItem> = Vec::with_capacity(items.len());
    {
        let mut seen_paths: HashSet<&str> = HashSet::with_capacity(items.len());
        for item in items {
            if seen_paths.insert(&item.relative_path) {
                unique_items.push(item);
            }
        }
    }

    let filenames: Vec<String> = unique_items
        .iter()
        .map(|item| basename(&item.filename))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let mut tx = pool.begin().await?;
    // Advisory lock: two takeout registrations for the same user arriving
    // concurrently must not both claim the same filename.
    sqlx::query!(
        "SELECT pg_advisory_xact_lock($1)",
        TAKEOUT_LOCK_OFFSET + i64::from(user_id)
    )
    .execute(&mut *tx)
    .await?;

    // Clean out permanently failed pending links so their filenames can be
    // re-imported.
    TakeoutLinkStore::delete_stale_pending(&mut *tx, user_id).await?;

    let (existing_items, linked_items) =
        fetch_known_filenames(&mut tx, user_id, &filenames).await?;

    // Resolve (or create) the album per folder that contains media files.
    let mut album_ids: HashMap<String, String> = HashMap::new();
    let folders: HashSet<&str> = unique_items
        .iter()
        .filter_map(|item| item.folder.as_deref())
        .collect();
    for folder in folders {
        let meta = album_metadata.get(folder);
        let title = meta
            .and_then(|m| m.title.as_deref().map(str::trim))
            .filter(|t| !t.is_empty());
        let album_name = title.unwrap_or(folder);
        let album = AlbumStore::find_or_create_takeout_album(
            &mut tx,
            user_id,
            album_name,
            meta.and_then(|m| m.description.as_deref()),
            batch_id,
        )
        .await?;
        album_ids.insert(folder.to_owned(), album.id);
    }

    debug!("Takeout registry albums: {:?}", album_ids);

    let (pending_per_album, new_paths, redundant) = decide_item_fates(
        &mut tx,
        ingest,
        user_id,
        batch_id,
        &unique_items,
        &existing_items,
        &linked_items,
        &album_ids,
    )
    .await?;

    for (album_id, filenames) in &pending_per_album {
        TakeoutLinkStore::insert_pending(&mut *tx, user_id, batch_id, album_id, filenames).await?;
    }

    // Ingest jobs are inserted inside the same transaction as the links, so a
    // crash can never leave links pending without a pending pipeline for the
    // canonical file.
    bulk_enqueue_full_ingest_in_tx(&mut tx, ingest, &new_paths, user_id).await?;

    tx.commit().await?;
    Ok(redundant)
}

/// Batch-loads what is already known about each filename: existing media items
/// in the user's library, and filenames claimed by an earlier takeout pass
/// (possibly still pending ingestion).
async fn fetch_known_filenames(
    tx: &mut sqlx::PgTransaction<'_>,
    user_id: i32,
    filenames: &[String],
) -> color_eyre::Result<(
    HashMap<String, MediaItemIdAndFilename>,
    HashMap<String, Option<String>>,
)> {
    let mut existing_items: HashMap<String, MediaItemIdAndFilename> = HashMap::new();
    for chunk in filenames.chunks(1000) {
        for row in MediaItemStore::find_ids_by_filenames(&mut **tx, user_id, chunk).await? {
            existing_items.insert(row.filename.clone(), row);
        }
    }

    let mut linked_items: HashMap<String, Option<String>> = HashMap::new();
    for chunk in filenames.chunks(1000) {
        for row in TakeoutLinkStore::find_by_filenames(&mut **tx, user_id, chunk).await? {
            let entry = linked_items.entry(row.filename).or_insert(None);
            if entry.is_none()
                && let Some(item_id) = row.media_item_id
            {
                *entry = Some(item_id);
            }
        }
    }
    Ok((existing_items, linked_items))
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
async fn decide_item_fates(
    tx: &mut sqlx::PgTransaction<'_>,
    ingest: &IngestSettings,
    user_id: i32,
    batch_id: &str,
    items: &[&ExtractedItem],
    existing_items: &HashMap<String, MediaItemIdAndFilename>,
    linked_items: &HashMap<String, Option<String>>,
    album_ids: &HashMap<String, String>,
) -> color_eyre::Result<(HashMap<String, Vec<String>>, Vec<String>, Vec<PathBuf>)> {
    let mut claimed_in_zip: HashSet<String> = HashSet::new();
    let mut pending_per_album: HashMap<String, Vec<String>> = HashMap::new();
    let mut new_paths: Vec<String> = Vec::new();
    let mut redundant: Vec<PathBuf> = Vec::new();

    for item in items {
        let filename = basename(&item.filename);
        let destination = ingest.media_root.join(&item.relative_path);
        let album_id = item.folder.as_ref().and_then(|f| album_ids.get(f)).cloned();

        if let Some(existing) = existing_items.get(&filename) {
            // Already in the library: keep only if this IS that copy on disk.
            let same_path = existing.relative_path == item.relative_path;
            if !same_path {
                redundant.push(destination);
            }
            if let Some(album_id) = album_id {
                TakeoutLinkStore::insert_consumed(
                    &mut **tx,
                    user_id,
                    batch_id,
                    &album_id,
                    &filename,
                    &existing.id,
                )
                .await?;
                attach_media_item_to_albums(
                    tx,
                    user_id,
                    &existing.id,
                    std::slice::from_ref(&album_id),
                )
                .await?;
            }
        } else if let Some(linked_item) = linked_items.get(&filename).cloned().flatten() {
            // Claimed by an earlier takeout pass that is still ingesting: link
            // this album to that existing item and drop the duplicate copy.
            redundant.push(destination);
            if let Some(album_id) = album_id {
                TakeoutLinkStore::insert_consumed(
                    &mut **tx,
                    user_id,
                    batch_id,
                    &album_id,
                    &filename,
                    &linked_item,
                )
                .await?;
                attach_media_item_to_albums(
                    tx,
                    user_id,
                    &linked_item,
                    std::slice::from_ref(&album_id),
                )
                .await?;
            }
        } else if !claimed_in_zip.insert(filename.clone()) {
            // Second (or later) occurrence of the same filename in this zip.
            redundant.push(destination);
            if let Some(album_id) = album_id {
                pending_per_album
                    .entry(album_id)
                    .or_default()
                    .push(filename);
            }
        } else {
            // Canonical copy: keep the file and start the ingest pipeline.
            if let Some(album_id) = album_id {
                pending_per_album
                    .entry(album_id)
                    .or_default()
                    .push(filename);
            }
            new_paths.push(item.relative_path.clone());
        }
    }

    Ok((pending_per_album, new_paths, redundant))
}

fn basename(filename: &str) -> String {
    filename.rsplit('/').next().unwrap_or(filename).to_owned()
}
