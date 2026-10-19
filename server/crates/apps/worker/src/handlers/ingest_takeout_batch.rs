use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::require_media_mounted;
use app_state::MakeRelativePath;
use app_state::constants::{TAKEOUT_IMPORT_FOLDER, TUS_UPLOADS_FOLDER};
use chrono::{DateTime, Utc};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::{IngestMetadataPayload, enqueue_full_ingest};
use common_services::utils::write_media_datetime;
use sanitize_filename::sanitize;
use serde::{Deserialize};
use serde_json::from_value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use common_services::api::upload::interfaces::TakeoutBatchPayload;

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
#[allow(dead_code)]
struct SidecarJson {
    description: Option<String>,
    #[serde(rename = "photoTakenTime")]
    photo_taken_time: Option<SidecarTimestamp>,
    #[serde(rename = "creationTime")]
    creation_time: Option<SidecarTimestamp>,
}

async fn get_unique_filename(dest_folder: &Path, destination_path: &Path) -> PathBuf {
    let mut unique_destination_path = destination_path.to_path_buf();
    let mut counter = 1;

    while fs::try_exists(&unique_destination_path)
        .await
        .unwrap_or(false)
    {
        let stem = destination_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let extension = destination_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy();

        let new_filename = if extension.is_empty() {
            format!("{stem}_{counter}")
        } else {
            format!("{stem}_{counter}.{extension}")
        };

        unique_destination_path = dest_folder.join(new_filename);
        counter += 1;
    }

    unique_destination_path
}

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

    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);
    let extract_dir = context
        .settings
        .ingest
        .cache_root
        .join("takeout_tmp")
        .join(&payload.batch_id);

    tokio::fs::create_dir_all(&extract_dir).await?;

    info!(
        "📦 Extracting {} Takeout ZIP archive(s) for user {} to {:?}",
        payload.tus_upload_ids.len(),
        user_id,
        extract_dir
    );

    // Extract all ZIPs into the shared extraction folder
    for upload_id in &payload.tus_upload_ids {
        let zip_path = tus_dir.join(upload_id);
        if tokio::fs::try_exists(&zip_path).await.unwrap_or(false) {
            let extract_clone = extract_dir.clone();
            tokio::task::spawn_blocking(move || extract_zip_archive(&zip_path, &extract_clone))
                .await??;
        } else {
            warn!("Takeout ZIP part not found: {:?}", zip_path);
        }
    }

    // Locate the root Google Photos directory inside the extracted tree
    let mut photos_root = extract_dir.clone();
    if photos_root.join("Takeout").join("Google Photos").exists() {
        photos_root = photos_root.join("Takeout").join("Google Photos");
    } else if photos_root.join("Google Photos").exists() {
        photos_root = photos_root.join("Google Photos");
    }

    info!("🔍 Scanning extracted Takeout folders at {:?}", photos_root);

    // 1. Scan albums and collect album info
    let mut album_name_to_id: HashMap<String, String> = HashMap::new();
    let mut folder_to_album_id: HashMap<PathBuf, String> = HashMap::new();

    let mut dir_entries = tokio::fs::read_dir(&photos_root).await?;
    while let Some(entry) = dir_entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let folder_name = entry.file_name().to_string_lossy().trim().to_string();
            if folder_name.is_empty() {
                continue;
            }

            // Check if metadata.json exists in this folder
            let meta_path = path.join("metadata.json");
            let mut album_title = folder_name.clone();
            let mut album_desc = None;

            if let Ok(meta_content) = tokio::fs::read_to_string(&meta_path).await
                && let Ok(album_meta) = serde_json::from_str::<AlbumMetadataJson>(&meta_content) {
                    if let Some(t) = album_meta.title {
                        let trimmed = t.trim().to_string();
                        if !trimmed.is_empty() {
                            album_title = trimmed;
                        }
                    }
                    if let Some(d) = album_meta.description {
                        let trimmed = d.trim().to_string();
                        if !trimmed.is_empty() {
                            album_desc = Some(trimmed);
                        }
                    }
                }

            // Find or create album for user
            let album_id = if let Some(existing_id) = album_name_to_id.get(&album_title) {
                existing_id.clone()
            } else {
                let existing_album =
                    AlbumStore::find_by_name(&context.pool, user_id, &album_title).await?;
                let id = if let Some(album) = existing_album {
                    album.id
                } else {
                    let new_id = common_services::utils::nice_id(10);
                    let payload = common_services::database::CreateAlbumPayload {
                        owner_id: user_id,
                        name: album_title.clone(),
                        description: album_desc,
                        thumbnail_id: None,
                        sort_mode: common_services::api::album::interfaces::AlbumSort::None,
                        is_public: false,
                    };
                    let created = AlbumStore::create(&context.pool, &new_id, payload).await?;
                    created.id
                };
                album_name_to_id.insert(album_title.clone(), id.clone());
                id
            };

            folder_to_album_id.insert(path, album_id);
        }
    }

    // 2. Index sidecar files & media files across all folders
    // Mapping: (folder_path, sanitized_filename) -> sidecar_path
    let mut sidecar_map: HashMap<(PathBuf, String), PathBuf> = HashMap::new();
    // Mapping: sanitized_filename -> (first_file_path, HashSet<album_id>)
    let mut media_file_map: HashMap<String, (PathBuf, HashSet<String>)> = HashMap::new();

    for (folder_path, album_id) in &folder_to_album_id {
        let mut entries = tokio::fs::read_dir(folder_path).await?;
        while let Some(file_entry) = entries.next_entry().await? {
            let fpath = file_entry.path();
            if !fpath.is_file() {
                continue;
            }
            let fname = file_entry.file_name().to_string_lossy().to_string();

            if Path::new(&fname)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json")) {
                // Determine target file stem
                let base_name = if let Some(stripped) = fname.strip_suffix(".supplemental-metadata.json") {
                    stripped
                } else if let Some(stripped) = fname.strip_suffix(".json") {
                    stripped
                } else {
                    continue;
                };

                let target_name = sanitize(base_name);
                sidecar_map.insert((folder_path.clone(), target_name), fpath);
            } else if context.settings.ingest.is_media_file(&fpath) {
                let sanitized = sanitize(&fname);
                let entry = media_file_map
                    .entry(sanitized)
                    .or_insert_with(|| (fpath.clone(), HashSet::new()));
                entry.1.insert(album_id.clone());
            }
        }
    }

    let all_media_filenames: Vec<String> = media_file_map.keys().cloned().collect();
    info!(
        "📋 Found {} unique media item(s) across Takeout albums",
        all_media_filenames.len()
    );

    // 3. Batch deduplication against DB
    let existing_items_map =
        MediaItemStore::find_existing_filenames_map(&context.pool, user_id, &all_media_filenames)
            .await?;

    let user_takeout_folder = context
        .settings
        .ingest
        .media_root
        .join(&user_media_folder)
        .join(TAKEOUT_IMPORT_FOLDER);
    tokio::fs::create_dir_all(&user_takeout_folder).await?;

    let mut newly_imported_count = 0;
    let mut linked_existing_count = 0;

    for (filename, (src_file_path, album_ids)) in media_file_map {
        let album_ids_vec: Vec<String> = album_ids.into_iter().collect();

        if let Some(existing_media_id) = existing_items_map.get(&filename) {
            // Already imported: attach to albums
            for alb_id in &album_ids_vec {
                AlbumStore::add_media_items(
                    &context.pool,
                    alb_id,
                    std::slice::from_ref(existing_media_id),
                    user_id,
                )
                .await?;
            }
            linked_existing_count += 1;
        } else {
            // New file: check EXIF confidence and write sidecar datetime if needed
            let is_video = context.settings.ingest.is_video_file(&src_file_path);
            let sidecar_opt = src_file_path
                .parent()
                .and_then(|p| sidecar_map.get(&(p.to_path_buf(), filename.clone())));

            let needs_sidecar_date = match context
                .media_analyzer
                .analyze_media(&src_file_path)
                .await
            {
                Ok(info) => {
                    let is_low_confidence =
                        info.time.source_details.confidence.eq_ignore_ascii_case("low");
                    let is_future = info.time.datetime_utc.is_some_and(|dt| dt > Utc::now());
                    let is_epoch_zero = info
                        .time
                        .datetime_utc
                        .is_some_and(|dt| dt.timestamp() == 0)
                        || (info.time.datetime_local.and_utc().timestamp() == 0);

                    is_low_confidence || is_future || is_epoch_zero
                }
                Err(_) => true,
            };

            if needs_sidecar_date && let Some(sidecar_path) = sidecar_opt
                && let Some(sidecar_dt) = parse_sidecar_timestamp(sidecar_path) {
                    let formatted = sidecar_dt.format("%Y:%m:%d %H:%M:%S").to_string();
                    let target_path = src_file_path.clone();
                    let res = tokio::task::spawn_blocking(move || {
                        write_media_datetime(&target_path, &formatted, is_video)
                    })
                    .await?;

                    if let Err(e) = res {
                        debug!("Failed to write sidecar datetime to {:?}: {}", src_file_path, e);
                    }
                }

            // Move to user's takeout managed folder
            let dest_file_path = get_unique_filename(
                &user_takeout_folder,
                &user_takeout_folder.join(&filename),
            )
            .await;

            if let Err(_e) = tokio::fs::rename(&src_file_path, &dest_file_path).await {
                // If cross-device move, copy and delete
                tokio::fs::copy(&src_file_path, &dest_file_path).await?;
                let _ = tokio::fs::remove_file(&src_file_path).await;
            }

            // Enqueue full ingest pipeline
            let rel_path =
                dest_file_path.make_relative(&context.settings.ingest.media_root)?;
            enqueue_full_ingest(
                &context.pool,
                &context.settings.ingest,
                &rel_path,
                user_id,
                Some(IngestMetadataPayload {
                    album_ids: album_ids_vec,
                    remote_user_identity: None,
                }),
            )
            .await?;

            newly_imported_count += 1;
        }
    }

    info!(
        "✅ Takeout batch finished: {} existing media linked to albums, {} new media enqueued for ingest",
        linked_existing_count, newly_imported_count
    );

    // Cleanup: remove temporary extraction folder and staged TUS zips
    let _ = tokio::fs::remove_dir_all(&extract_dir).await;
    for upload_id in &payload.tus_upload_ids {
        let zip_path = tus_dir.join(upload_id);
        let info_path = tus_dir.join(format!("{upload_id}.info"));
        let _ = tokio::fs::remove_file(zip_path).await;
        let _ = tokio::fs::remove_file(info_path).await;
    }

    Ok(JobResult::Done)
}
