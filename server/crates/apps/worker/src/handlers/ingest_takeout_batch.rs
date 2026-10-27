use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::{get_unique_filename, require_media_mounted};
use app_state::MakeRelativePath;
use app_state::constants::{TAKEOUT_IMPORT_FOLDER, TUS_UPLOADS_FOLDER};
use chrono::{DateTime, Utc};
use color_eyre::Result;
use color_eyre::eyre::{bail, eyre};
use common_services::api::upload::interfaces::TakeoutBatchPayload;
use common_services::api::upload::service::repair_tus_upload;
use common_services::database::album::album::AlbumRole;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::{IngestMetadataPayload, bulk_enqueue_full_ingest};
use sanitize_filename::sanitize;
use serde::Deserialize;
use serde_json::from_value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::info;

#[derive(Debug, Deserialize)]
struct AlbumMetadataJson {
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SidecarTimestamp {
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SidecarJson {
    #[serde(rename = "photoTakenTime")]
    photo_taken_time: Option<SidecarTimestamp>,
    #[serde(rename = "creationTime")]
    creation_time: Option<SidecarTimestamp>,
}

/// Folder -> sidecar file name -> sidecar path.
type SidecarIndex = HashMap<PathBuf, HashMap<String, PathBuf>>;
/// Media file name -> (first seen path, album ids that contain it).
type MediaIndex = HashMap<String, (PathBuf, HashSet<String>)>;

fn extract_zip_archive(zip_path: &Path, extract_to: &Path) -> Result<()> {
    let file = File::open(zip_path)?;
    let reader = BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let raw_path = match file.enclosed_name() {
            Some(p) => p.clone(),
            None => continue,
        };

        // Normalize path and trim components to remove trailing spaces Google Takeout adds
        let mut normalized = PathBuf::new();
        for comp in raw_path.components() {
            let s = comp.as_os_str().to_string_lossy();
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                normalized.push(trimmed);
            }
        }

        let outpath = extract_to.join(normalized);

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

fn parse_sidecar_timestamp(sidecar_path: &Path) -> Option<DateTime<Utc>> {
    let content = std::fs::read_to_string(sidecar_path).ok()?;
    let parsed: SidecarJson = serde_json::from_str(&content).ok()?;

    let ts_str = parsed
        .photo_taken_time
        .and_then(|t| t.timestamp)
        .or_else(|| parsed.creation_time.and_then(|t| t.timestamp))?;

    let epoch_secs = ts_str.parse::<i64>().ok()?;
    DateTime::from_timestamp(epoch_secs, 0)
}

/// Splits a trailing Takeout duplicate counter, e.g. `IMG_0001(2)` -> (`IMG_0001`, `2`).
fn split_duplicate_counter(stem: &str) -> (&str, Option<&str>) {
    if let Some(open) = stem.rfind('(')
        && let Some(counter) = stem[open + 1..].strip_suffix(')')
        && !counter.is_empty()
        && counter.bytes().all(|b| b.is_ascii_digit())
    {
        return (&stem[..open], Some(counter));
    }
    (stem, None)
}

/// Candidate sidecar names for a media file, most specific first.
///
/// Takeout does not always name the sidecar after the media file exactly: duplicate counters
/// may live on either side, the media extension can be omitted, and edited copies often
/// inherit the original's sidecar.
fn sidecar_candidates(filename: &str) -> Vec<String> {
    let path = Path::new(filename);
    let (Some(stem), Some(ext)) = (
        path.file_stem().and_then(|s| s.to_str()),
        path.extension().and_then(|e| e.to_str()),
    ) else {
        return Vec::new();
    };

    let (base_stem, counter) = split_duplicate_counter(stem);
    let mut candidates = vec![
        format!("{filename}.supplemental-metadata.json"),
        format!("{filename}.json"),
    ];

    if let Some(counter) = counter {
        candidates.push(format!(
            "{base_stem}.{ext}.supplemental-metadata({counter}).json"
        ));
        candidates.push(format!("{base_stem}.supplemental-metadata({counter}).json"));
    }

    candidates.push(format!("{base_stem}.{ext}.supplemental-metadata.json"));
    candidates.push(format!("{base_stem}.supplemental-metadata.json"));
    candidates.push(format!("{base_stem}.{ext}.json"));
    candidates.push(format!("{base_stem}.json"));

    if let Some(edited_base) = base_stem.strip_suffix("-edited") {
        candidates.push(format!("{edited_base}.{ext}.supplemental-metadata.json"));
        candidates.push(format!("{edited_base}.{ext}.json"));
        candidates.push(format!("{edited_base}.supplemental-metadata.json"));
    }

    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn find_sidecar<'a>(
    sidecars: &'a SidecarIndex,
    folder: &Path,
    filename: &str,
) -> Option<&'a PathBuf> {
    let folder_sidecars = sidecars.get(folder)?;
    // Takeout is inconsistent about extension case (`google.PNG` vs `google.png....json`).
    sidecar_candidates(filename)
        .into_iter()
        .find_map(|candidate| folder_sidecars.get(&candidate.to_lowercase()))
}

fn locate_photos_root(extract_dir: &Path) -> PathBuf {
    let with_takeout = extract_dir.join("Takeout").join("Google Photos");
    if with_takeout.exists() {
        return with_takeout;
    }
    let without_takeout = extract_dir.join("Google Photos");
    if without_takeout.exists() {
        return without_takeout;
    }
    extract_dir.to_path_buf()
}

async fn extract_archives(
    context: &WorkerContext,
    payload: &TakeoutBatchPayload,
    extract_dir: &Path,
) -> Result<()> {
    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);

    for upload_id in &payload.tus_upload_ids {
        repair_tus_upload(&tus_dir, upload_id).await?;
        let zip_path = tus_dir.join(upload_id);
        if fs::try_exists(&zip_path).await.unwrap_or(false) {
            let extract_to = extract_dir.to_path_buf();
            tokio::task::spawn_blocking(move || extract_zip_archive(&zip_path, &extract_to))
                .await??;
        } else {
            // A partial batch would silently drop albums; fail so the uploads can be retried.
            bail!("Takeout ZIP part {:?} is missing", zip_path);
        }
    }
    Ok(())
}

/// Creates an album for every top-level Takeout folder and returns folder -> album id.
async fn create_albums(
    context: &WorkerContext,
    user_id: i32,
    photos_root: &Path,
) -> Result<HashMap<PathBuf, String>> {
    let mut album_name_to_id: HashMap<String, String> = HashMap::new();
    let mut folder_to_album_id: HashMap<PathBuf, String> = HashMap::new();

    let mut dir_entries = fs::read_dir(photos_root).await?;
    while let Some(entry) = dir_entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = entry.file_name().to_string_lossy().trim().to_string();
        if folder_name.is_empty() {
            continue;
        }

        let mut album_title = folder_name;
        let mut album_desc = None;
        if let Ok(meta_content) = fs::read_to_string(path.join("metadata.json")).await
            && let Ok(album_meta) = serde_json::from_str::<AlbumMetadataJson>(&meta_content)
        {
            if let Some(title) = album_meta.title {
                let trimmed = title.trim().to_string();
                if !trimmed.is_empty() {
                    album_title = trimmed;
                }
            }
            if let Some(description) = album_meta.description {
                let trimmed = description.trim().to_string();
                if !trimmed.is_empty() {
                    album_desc = Some(trimmed);
                }
            }
        }

        let album_id = if let Some(existing_id) = album_name_to_id.get(&album_title) {
            existing_id.clone()
        } else {
            let existing_album =
                AlbumStore::find_by_name(&context.pool, user_id, &album_title).await?;
            let id = if let Some(album) = existing_album {
                AlbumStore::upsert_collaborator(
                    &context.pool,
                    &album.id,
                    user_id,
                    AlbumRole::Owner,
                )
                .await?;
                album.id
            } else {
                let new_id = common_services::utils::nice_id(10);
                let payload = common_services::database::CreateAlbumPayload {
                    owner_id: user_id,
                    name: album_title.clone(),
                    description: album_desc,
                    thumbnail_id: None,
                    sort_mode: common_services::api::album::interfaces::AlbumSort::DateAsc,
                    is_public: false,
                };
                let mut tx = context.pool.begin().await?;
                let album = AlbumStore::create(&mut *tx, &new_id, payload).await?;
                AlbumStore::upsert_collaborator(&mut *tx, &album.id, user_id, AlbumRole::Owner)
                    .await?;
                tx.commit().await?;
                album.id
            };
            album_name_to_id.insert(album_title, id.clone());
            id
        };

        folder_to_album_id.insert(path, album_id);
    }

    Ok(folder_to_album_id)
}

async fn index_files(
    context: &WorkerContext,
    folder_to_album_id: &HashMap<PathBuf, String>,
) -> Result<(SidecarIndex, MediaIndex)> {
    let mut sidecars: SidecarIndex = HashMap::new();
    let mut media_files: MediaIndex = HashMap::new();

    for (folder_path, album_id) in folder_to_album_id {
        let mut entries = fs::read_dir(folder_path).await?;
        while let Some(file_entry) = entries.next_entry().await? {
            let file_path = file_entry.path();
            if !file_path.is_file() {
                continue;
            }
            let file_name = file_entry.file_name().to_string_lossy().to_string();

            if Path::new(&file_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                sidecars
                    .entry(folder_path.clone())
                    .or_default()
                    .insert(sanitize(&file_name).to_lowercase(), file_path);
            } else if context.settings.ingest.is_media_file(&file_path) {
                let sanitized = sanitize(&file_name);
                media_files
                    .entry(sanitized)
                    .or_insert_with(|| (file_path.clone(), HashSet::new()))
                    .1
                    .insert(album_id.clone());
            }
        }
    }

    Ok((sidecars, media_files))
}

async fn import_media_item(
    context: &WorkerContext,
    dest_folder: &Path,
    filename: &str,
    source_path: &Path,
    sidecar_path: Option<&Path>,
) -> Result<(String, Option<DateTime<Utc>>)> {
    let sidecar_taken_at = sidecar_path.and_then(parse_sidecar_timestamp);

    // Move to user's takeout managed folder
    let dest_file_path = get_unique_filename(dest_folder, &dest_folder.join(filename)).await;

    if tokio::fs::rename(source_path, &dest_file_path)
        .await
        .is_err()
    {
        // If cross-device move, copy and delete
        tokio::fs::copy(source_path, &dest_file_path).await?;
        let _ = tokio::fs::remove_file(source_path).await;
    }

    let relative_path = dest_file_path.make_relative(&context.settings.ingest.media_root)?;
    Ok((relative_path, sidecar_taken_at))
}

async fn cleanup(context: &WorkerContext, payload: &TakeoutBatchPayload, extract_dir: &Path) {
    let _ = fs::remove_dir_all(extract_dir).await;

    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);
    for upload_id in &payload.tus_upload_ids {
        let _ = fs::remove_file(tus_dir.join(upload_id)).await;
        let _ = fs::remove_file(tus_dir.join(format!("{upload_id}.info"))).await;
    }
}

#[allow(clippy::too_many_lines)]
pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    if let Some(result) =
        require_media_mounted(&context.pool, &context.settings.ingest.media_root).await?
    {
        return Ok(result);
    }

    let user_id = job
        .user_id
        .ok_or_else(|| eyre!("IngestTakeoutBatch missing user_id"))?;
    let user_media_folder = UserStore::get_user_media_folder(&context.pool, user_id)
        .await?
        .ok_or_else(|| eyre!("IngestTakeoutBatch: User has no media folder"))?;

    let payload_val = job
        .payload
        .as_ref()
        .ok_or_else(|| eyre!("IngestTakeoutBatch missing payload"))?;
    let payload: TakeoutBatchPayload = from_value(payload_val.clone())?;

    let extract_dir = context
        .settings
        .ingest
        .cache_root
        .join("takeout_tmp")
        .join(job.id.to_string());
    fs::create_dir_all(&extract_dir).await?;

    info!(
        "📦 Extracting {} Takeout ZIP archive(s) for user {} to {:?}",
        payload.tus_upload_ids.len(),
        user_id,
        extract_dir
    );
    extract_archives(context, &payload, &extract_dir).await?;
    info!(
        "✅ Extracted {} Takeout ZIP archive(s)",
        payload.tus_upload_ids.len()
    );

    let photos_root = locate_photos_root(&extract_dir);
    info!("🔍 Scanning extracted Takeout folders at {:?}", photos_root);

    let folder_to_album_id = create_albums(context, user_id, &photos_root).await?;
    let (sidecars, media_files) = index_files(context, &folder_to_album_id).await?;

    let all_media_filenames: Vec<String> = media_files.keys().cloned().collect();
    info!(
        "📋 Found {} unique media item(s) across Takeout albums",
        all_media_filenames.len()
    );

    let existing_items_map =
        MediaItemStore::find_existing_filenames_map(&context.pool, user_id, &all_media_filenames)
            .await?;

    let user_takeout_folder = context
        .settings
        .ingest
        .media_root
        .join(&user_media_folder)
        .join(TAKEOUT_IMPORT_FOLDER);
    fs::create_dir_all(&user_takeout_folder).await?;

    let mut newly_imported_count = 0;
    let mut linked_existing_count = 0;
    let mut new_media = Vec::new();
    let mut existing_album_links: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let total_media_count = media_files.len();
    let progress_interval = Duration::from_secs(10);
    let mut last_progress = Instant::now();

    info!("📥 Importing {total_media_count} Takeout media item(s)...");

    for (index, (filename, (source_path, album_ids))) in media_files.into_iter().enumerate() {
        let mut album_ids: Vec<String> = album_ids.into_iter().collect();
        album_ids.sort_unstable();

        if let Some(existing_media_id) = existing_items_map.get(&filename) {
            // Already imported: only attach to the albums, ignore its sidecar.
            for album_id in album_ids {
                existing_album_links
                    .entry(album_id)
                    .or_default()
                    .push(existing_media_id.clone());
            }
            linked_existing_count += 1;
        } else {
            let sidecar_path = source_path
                .parent()
                .and_then(|folder| find_sidecar(&sidecars, folder, &filename))
                .map(PathBuf::as_path);

            let (relative_path, fallback_taken_at) = import_media_item(
                context,
                &user_takeout_folder,
                &filename,
                &source_path,
                sidecar_path,
            )
            .await?;

            new_media.push((
                relative_path,
                Some(IngestMetadataPayload {
                    album_ids,
                    remote_user_identity: None,
                    fallback_taken_at,
                }),
            ));
            newly_imported_count += 1;
        }

        let processed = index + 1;
        // Time-based keeps slow imports informative; count-based covers fast ones.
        if processed < total_media_count
            && (processed % 1000 == 0 || last_progress.elapsed() >= progress_interval)
        {
            info!(
                "📥 Takeout progress: {processed}/{total_media_count} media item(s) processed ({newly_imported_count} new, {linked_existing_count} existing)"
            );
            last_progress = Instant::now();
        }
    }

    if !existing_album_links.is_empty() {
        info!(
            "🔗 Linking {linked_existing_count} existing media item(s) to {} album(s)...",
            existing_album_links.len()
        );
    }
    for (album_id, media_item_ids) in existing_album_links {
        let mut tx = context.pool.begin().await?;
        AlbumStore::add_media_items(&mut tx, &album_id, &media_item_ids, user_id).await?;
        tx.commit().await?;
    }

    if !new_media.is_empty() {
        bulk_enqueue_full_ingest(&context.pool, &context.settings.ingest, &new_media, user_id)
            .await?;
    }

    info!(
        "✅ Takeout batch finished: {} existing media linked to albums, {} new media enqueued for ingest",
        linked_existing_count, newly_imported_count
    );

    cleanup(context, &payload, &extract_dir).await;

    Ok(JobResult::Done)
}

#[cfg(test)]
mod tests {
    use super::{SidecarIndex, find_sidecar, sidecar_candidates};
    use std::path::PathBuf;

    #[test]
    fn sidecar_candidates_cover_takeout_naming_quirks() {
        assert_eq!(
            sidecar_candidates("photo.jpg")[0],
            "photo.jpg.supplemental-metadata.json"
        );
        // Counter on the media file and on the sidecar.
        assert!(
            sidecar_candidates("2015-03-22(2).jpg")
                .contains(&"2015-03-22.jpg.supplemental-metadata(2).json".to_owned())
        );
        // Counter only on the sidecar, without the media extension.
        assert!(
            sidecar_candidates("2013-08-11(1).png")
                .contains(&"2013-08-11.supplemental-metadata(1).json".to_owned())
        );
        // Sidecar without the media extension.
        assert!(sidecar_candidates("bg.png").contains(&"bg.supplemental-metadata.json".to_owned()));
        // Edited copies fall back to the original's sidecar.
        assert!(
            sidecar_candidates("photo-edited.jpg")
                .contains(&"photo.jpg.supplemental-metadata.json".to_owned())
        );
        // The exact sidecar always wins over the counter variants.
        assert_eq!(
            sidecar_candidates("photo(1).jpg")[0],
            "photo(1).jpg.supplemental-metadata.json"
        );
    }

    #[test]
    fn sidecar_lookup_ignores_extension_case() {
        let folder = PathBuf::from("Photos from 2013");
        let mut sidecars = SidecarIndex::new();
        sidecars.entry(folder.clone()).or_default().insert(
            "google.png.supplemental-metadata.json".to_owned(),
            folder.join("google.PNG.supplemental-metadata.json"),
        );

        // `google-edited.png` inherits the sidecar of `google.PNG`.
        assert!(find_sidecar(&sidecars, &folder, "google-edited.png").is_some());
    }
}
