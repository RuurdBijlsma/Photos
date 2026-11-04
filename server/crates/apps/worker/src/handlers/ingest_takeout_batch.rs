mod archive;

use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::utils::require_media_mounted;
use app_state::MakeRelativePath;
use app_state::constants::{TAKEOUT_IMPORT_FOLDER, TUS_UPLOADS_FOLDER};
use color_eyre::Result;
use color_eyre::eyre::{bail, eyre};
use common_services::api::upload::interfaces::TakeoutBatchPayload;
use common_services::api::upload::service::repair_tus_upload;
use common_services::database::album::album::AlbumRole;
use common_services::database::album_store::AlbumStore;
use common_services::database::jobs::{Job, JobType};
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::user_store::UserStore;
use common_services::job_queue::{
    IngestMetadataPayload, TakeoutAlbumImportPayload, bulk_enqueue_full_ingest, enqueue_job,
};
use serde_json::from_value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use tokio::fs;
use tracing::{info, warn};

/// Repairs the finished uploads and indexes their ZIPs without extracting any media.
async fn index_uploads(
    context: &WorkerContext,
    upload_ids: &[String],
    tus_dir: &Path,
) -> Result<archive::TakeoutIndex> {
    for upload_id in upload_ids {
        repair_tus_upload(tus_dir, upload_id).await?;
        let zip_path = tus_dir.join(upload_id);
        if !fs::try_exists(&zip_path).await.unwrap_or(false) {
            // A partial batch would silently drop albums; fail so the uploads can be retried.
            bail!("Takeout ZIP part {zip_path:?} is missing");
        }
    }

    let mut index = archive::TakeoutIndex::default();
    for upload_id in upload_ids {
        let zip_path = tus_dir.join(upload_id);
        let ingest = context.settings.ingest.clone();
        let upload_id = upload_id.clone();
        let partial = tokio::task::spawn_blocking(move || {
            let mut partial = archive::TakeoutIndex::default();
            archive::index_archive(&mut partial, &upload_id, &zip_path, |path| {
                ingest.is_media_file(path)
            })?;
            Ok::<_, color_eyre::Report>(partial)
        })
        .await??;
        index.merge(partial);
    }

    Ok(index)
}

/// Creates an album for every Takeout folder in the index and returns folder -> album id.
async fn create_albums(
    context: &WorkerContext,
    user_id: i32,
    albums: &BTreeMap<String, archive::AlbumMetadata>,
) -> Result<HashMap<String, String>> {
    let mut album_name_to_id: HashMap<String, String> = HashMap::new();
    let mut folder_to_album_id: HashMap<String, String> = HashMap::new();

    for (folder, metadata) in albums {
        let album_title = metadata.title.clone().unwrap_or_else(|| folder.clone());

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
                    description: metadata.description.clone(),
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

        folder_to_album_id.insert(folder.clone(), album_id);
    }

    Ok(folder_to_album_id)
}

async fn cleanup(context: &WorkerContext, payload: &TakeoutBatchPayload) {
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

    let tus_dir = context
        .settings
        .ingest
        .app_data_root
        .join(TUS_UPLOADS_FOLDER);

    info!(
        "📦 Indexing {} Takeout ZIP archive(s) for user {user_id}",
        payload.tus_upload_ids.len()
    );
    let index = index_uploads(context, &payload.tus_upload_ids, &tus_dir).await?;
    if index.media.is_empty() {
        bail!(
            "no media entries found in {} Takeout ZIP(s); expected Takeout/Google Photos/<album>/<file>",
            payload.tus_upload_ids.len()
        );
    }
    info!(
        "📋 Found {} album(s) and {} unique media item(s)",
        index.albums.len(),
        index.media.len()
    );

    let folder_to_album_id = create_albums(context, user_id, &index.albums).await?;

    let all_filenames: Vec<String> = index
        .media
        .values()
        .map(|media| media.filename.clone())
        .collect();
    let existing_items_map =
        MediaItemStore::find_existing_filenames_map(&context.pool, user_id, &all_filenames).await?;

    let user_takeout_folder = context
        .settings
        .ingest
        .media_root
        .join(&user_media_folder)
        .join(TAKEOUT_IMPORT_FOLDER);
    fs::create_dir_all(&user_takeout_folder).await?;

    let mut existing_album_links: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut album_ids_by_filename: HashMap<String, Vec<String>> = HashMap::new();
    let mut requests_by_upload: BTreeMap<String, Vec<archive::CopyRequest>> = BTreeMap::new();
    let mut linked_existing_count = 0;

    for media in index.media.values() {
        let mut album_ids: Vec<String> = media
            .albums
            .iter()
            .filter_map(|folder| folder_to_album_id.get(folder).cloned())
            .collect();
        album_ids.sort_unstable();

        if let Some(existing_media_id) = existing_items_map.get(&media.filename) {
            // Already imported: only attach to the albums, ignore its sidecar.
            for album_id in &album_ids {
                existing_album_links
                    .entry(album_id.clone())
                    .or_default()
                    .push(existing_media_id.clone());
            }
            linked_existing_count += 1;
            continue;
        }

        requests_by_upload
            .entry(media.entry.upload_id.clone())
            .or_default()
            .push(archive::CopyRequest {
                entry_index: media.entry.index,
                filename: media.filename.clone(),
                size: media.size,
                fallback_taken_at: archive::sidecar_timestamp(
                    &index,
                    &media.folder,
                    &media.filename,
                ),
            });
        album_ids_by_filename.insert(media.filename.clone(), album_ids);
    }

    let mut new_media = Vec::with_capacity(album_ids_by_filename.len());
    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut copied_bytes = 0_u64;

    for (upload_id, requests) in requests_by_upload {
        let total = requests.len();
        let zip_path = tus_dir.join(&upload_id);
        let dest_folder = user_takeout_folder.clone();
        let outcomes = tokio::task::spawn_blocking(move || {
            archive::copy_entries(&zip_path, &requests, &dest_folder)
        })
        .await??;

        let mut copied_here = 0;
        for outcome in outcomes {
            match outcome {
                archive::CopyOutcome::Copied {
                    filename,
                    size,
                    fallback_taken_at,
                } => {
                    let relative_path = user_takeout_folder
                        .join(&filename)
                        .make_relative(&context.settings.ingest.media_root)?;
                    new_media.push((
                        relative_path,
                        Some(IngestMetadataPayload {
                            album_ids: album_ids_by_filename.remove(&filename).unwrap_or_default(),
                            remote_user_identity: None,
                            defer_album_thumbnail: true,
                            fallback_taken_at,
                        }),
                    ));
                    copied_bytes += size;
                    copied_here += 1;
                }
                archive::CopyOutcome::Skipped { filename, reason } => {
                    warn!("Skipping {filename} from {upload_id}: {reason}");
                    skipped.push((upload_id.clone(), filename));
                }
            }
        }
        info!(
            "ZIP {upload_id}: copied {copied_here}/{total} entries, {} skipped, {copied_bytes} bytes",
            total - copied_here
        );
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

    let new_count = new_media.len();
    if !new_media.is_empty() {
        bulk_enqueue_full_ingest(&context.pool, &context.settings.ingest, &new_media, user_id)
            .await?;
    }

    let mut album_ids: Vec<String> = folder_to_album_id.into_values().collect();
    album_ids.sort_unstable();
    album_ids.dedup();
    if !album_ids.is_empty() {
        let import_payload = TakeoutAlbumImportPayload {
            album_ids,
            batch_job_id: job.id,
        };
        enqueue_job(
            &context.pool,
            &context.settings.ingest,
            JobType::PrepareTakeoutImport,
        )
        .user_id(user_id)
        .payload(&import_payload)
        .call()
        .await?;
        enqueue_job(
            &context.pool,
            &context.settings.ingest,
            JobType::FinalizeTakeoutImport,
        )
        .user_id(user_id)
        .payload(&import_payload)
        .call()
        .await?;
    }

    info!(
        "✅ Takeout batch finished: {linked_existing_count} existing media linked to albums, {new_count} new media enqueued for ingest ({copied_bytes} bytes copied), {} skipped",
        skipped.len()
    );
    if !skipped.is_empty() {
        let shown: Vec<String> = skipped
            .iter()
            .take(50)
            .map(|(zip, file)| format!("{zip}/{file}"))
            .collect();
        warn!(
            "⚠️ Skipped {} unreadable Takeout item(s) (first {}): {}",
            skipped.len(),
            shown.len(),
            shown.join(", ")
        );
    }

    cleanup(context, &payload).await;

    Ok(JobResult::Done)
}
