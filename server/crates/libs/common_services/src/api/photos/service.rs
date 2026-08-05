use crate::api::app_error::AppError;
use crate::api::photos::interfaces::{UpdateMediaItemRequest, UpdateMediaItemResponse};
use crate::database::app_user::UserRole;
use crate::database::cached_store::cached_store;
use crate::database::jobs::JobType;
use crate::database::media_item::media_item::CreateFullMediaItem;
use crate::database::media_item_store::MediaItemStore;
use crate::database::{UpdateField, UpdateMediaItemPayload, with_fallback_timezone};
use crate::job_queue::enqueue_job;
use crate::utils::{nice_id, write_exif_orientation};
use app_state::{IngestSettings, MakeRelativePath};
use axum::body::Body;
use axum_extra::headers::Range;
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use color_eyre::Report;
use color_eyre::eyre::eyre;
use exif::{In, Tag, Value};
use fast_image_resize as fr;
use http::{Response, StatusCode, header};
use image::ImageDecoder;
use image::ImageReader;
use sqlx::{Executor, PgPool, Postgres};
use std::io::Cursor;
use std::ops::Bound;
use std::path::Path;
use std::process::{Command, Stdio};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::{fs, task};
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::ReaderStream;
use tracing::{debug, warn};

/// Securely streams a validated media file to the client after performing authorization checks.
///
/// # Errors
///
/// Returns a `AppError` if path validation fails, the user is not authorized,
/// the file is not found, the media type is unsupported, or if any file system
/// or response building error occurs.
pub async fn download_media_file(
    ingestion: &IngestSettings,
    user_id: i32,
    user_role: UserRole,
    user_media_folder: Option<String>,
    path: &str,
) -> Result<Response<Body>, AppError> {
    // Path Validation
    let media_dir_canon = ingestion
        .media_root
        .canonicalize()
        .map_err(|e| Report::new(e).wrap_err("Failed to canonicalize media directory"))?;

    let file_path = ingestion.media_root.join(path);
    let file_canon = match file_path.canonicalize() {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!("File not found at path: {}", file_path.display());
            return Err(AppError::NotFound(path.to_owned()));
        }
        Err(e) => {
            return Err(Report::new(e)
                .wrap_err("Failed to canonicalize path")
                .into());
        }
    };

    if !file_canon.starts_with(&media_dir_canon) {
        warn!("Blocked directory traversal attempt for: {}", path);
        return Err(AppError::BadRequest("Invalid Path".to_owned()));
    }

    // Authorization
    let relative_path = file_canon.make_relative_canon(&ingestion.media_root_canon)?;
    if user_role != UserRole::Admin {
        let Some(user_media_folder) = &user_media_folder else {
            warn!(
                "Access denied for user {}: No media folder assigned.",
                user_id
            );
            return Err(AppError::Forbidden("Denied".to_owned()));
        };
        if !relative_path.starts_with(user_media_folder) {
            warn!(
                "Access denied for user {}: Attempted to access path outside their media folder.",
                user_id
            );
            return Err(AppError::Forbidden("Denied".to_owned()));
        }
    }

    // Get file
    if !ingestion.is_media_file(&file_canon) {
        warn!("Unsupported media type requested: {}", file_canon.display());
        return Err(AppError::UnsupportedMediaType(String::new()));
    }
    let file = match File::open(&file_canon).await {
        Ok(file) => file,
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Err(AppError::NotFound(path.to_owned())),
            std::io::ErrorKind::PermissionDenied => Err(AppError::Forbidden("Denied".to_owned())),
            _ => Err(Report::new(e).wrap_err("Failed to open media file").into()),
        }?,
    };

    let metadata = file
        .metadata()
        .await
        .map_err(|e| Report::new(e).wrap_err("Failed to read file metadata"))?;
    let file_size = metadata.len();

    // Streaming Response
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::from_stream(stream);
    let mime_type = mime_guess::from_path(&file_canon).first_or_octet_stream();
    let filename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mediafile");
    let disposition = format!("inline; filename=\"{filename}\"");
    let disposition_header = header::HeaderValue::from_str(&disposition)
        .unwrap_or(header::HeaderValue::from_static("inline"));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type.as_ref())
        .header(header::CONTENT_DISPOSITION, disposition_header)
        .header(header::CONTENT_LENGTH, file_size)
        .header(
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable",
        )
        .body(body)
        .map_err(|e| Report::new(e).wrap_err("Failed to build response"))?)
}

pub async fn thumbnail_on_demand_cached(
    pool: &PgPool,
    size: i32,
    media_item_id: &str,
    ingest_settings: &IngestSettings,
) -> Result<Vec<u8>, AppError> {
    let cache_dir = &ingest_settings.on_demand_thumbs_cache_root;
    let cache_filename = format!("{media_item_id}_{size}.jpg");
    let cache_path = cache_dir.join(&cache_filename);

    if let Ok(cached_data) = fs::read(&cache_path).await {
        return Ok(cached_data);
    }

    let Some(rel_path) = MediaItemStore::find_relative_path_by_id(pool, media_item_id).await?
    else {
        return Err(AppError::NotFound(media_item_id.to_owned()));
    };
    let media_path = ingest_settings.media_root.join(&rel_path);
    let is_video = ingest_settings.is_video_file(&media_path);

    let image_bytes = task::spawn_blocking(move || {
        if is_video {
            video_thumb_on_demand(&media_path, size)
        } else {
            image_thumb_on_demand(&media_path, size)
        }
    })
    .await
    .map_err(|e| AppError::Internal(eyre!("Task join error: {e}")))??;

    if let Err(e) = fs::write(&cache_path, &image_bytes).await {
        tracing::log::warn!("Failed to write thumbnail to cache: {e}");
    }

    Ok(image_bytes)
}

fn video_thumb_on_demand(path: &Path, target_size: i32) -> Result<Vec<u8>, AppError> {
    let seek_time = "0.5";

    let output = Command::new("ffmpeg")
        .args([
            "-ss",
            seek_time, // Fast seek (before -i)
            "-i",
            path.to_str()
                .ok_or(AppError::BadRequest("Invalid Path".to_owned()))?,
            "-vf",
            &format!("scale={target_size}:-2"), // Resize during decode
            "-frames:v",
            "1", // Extract exactly one frame
            "-f",
            "image2", // Force image output format
            "-c:v",
            "mjpeg", // Use JPEG encoder
            "-q:v",
            "4", // Quality scale (1-31, lower is better)
            "-update",
            "1",
            "pipe:1", // Output to stdout
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            AppError::Internal(eyre!(e).wrap_err("Failed to execute ffmpeg for video thumbnail"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If 0.5s failed (video might be very short), try again at 0s
        if stderr.contains("Output file is empty") && seek_time != "0" {
            return video_thumb_on_demand_at_zero(path, target_size);
        }
        return Err(AppError::Internal(eyre!("ffmpeg failed: {}", stderr)));
    }

    Ok(output.stdout)
}

fn video_thumb_on_demand_at_zero(path: &Path, target_size: i32) -> Result<Vec<u8>, AppError> {
    let output = Command::new("ffmpeg")
        .args([
            "-i",
            path.to_str()
                .ok_or(AppError::BadRequest("Invalid Path".to_owned()))?,
            "-vf",
            &format!("scale={target_size}:-2"),
            "-frames:v",
            "1",
            "-f",
            "image2",
            "-c:v",
            "mjpeg",
            "-q:v",
            "4",
            "pipe:1",
        ])
        .output()
        .map_err(|e| AppError::Internal(eyre!(e)))?;

    Ok(output.stdout)
}

fn image_thumb_on_demand(path: &Path, target_size: i32) -> Result<Vec<u8>, AppError> {
    let file_bytes = std::fs::read(path)
        .map_err(|e| AppError::Internal(eyre!(e).wrap_err("Failed to read image source")))?;

    let is_jpeg = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("jpeg"));
    if is_jpeg && let Some(thumb_bytes) = try_extract_exif_thumbnail(&file_bytes, target_size) {
        return Ok(thumb_bytes);
    }

    let thumb_bytes = quick_image_resize(path, &file_bytes, target_size)?;
    Ok(thumb_bytes)
}

fn try_extract_exif_thumbnail(file_bytes: &[u8], target_size: i32) -> Option<Vec<u8>> {
    let mut cursor = Cursor::new(file_bytes);
    let exif_reader = exif::Reader::new();
    let Ok(exif_data) = exif_reader.read_from_container(&mut cursor) else {
        return None;
    };

    let offset_field = exif_data.get_field(Tag::JPEGInterchangeFormat, In(1));
    let length_field = exif_data.get_field(Tag::JPEGInterchangeFormatLength, In(1));
    let (Some(off_f), Some(len_f)) = (offset_field, length_field) else {
        return None;
    };

    let off_val = match off_f.value {
        Value::Long(ref v) if !v.is_empty() => Some(v[0]),
        Value::Short(ref v) if !v.is_empty() => Some(u32::from(v[0])),
        _ => None,
    }?;

    let len_val = match len_f.value {
        Value::Long(ref v) if !v.is_empty() => Some(v[0]),
        Value::Short(ref v) if !v.is_empty() => Some(u32::from(v[0])),
        _ => None,
    }?;
    let start = off_val as usize;
    let end = start + len_val as usize;
    if end > file_bytes.len() {
        return None;
    }

    let thumb_bytes = &exif_data.buf()[start..end];
    let min_height = (target_size as f32 * 0.9) as usize;
    if let Ok(dim) = imagesize::blob_size(thumb_bytes)
        && (dim.height >= min_height)
    {
        return Some(thumb_bytes.to_vec());
    }

    None
}

fn quick_image_resize(path: &Path, file_bytes: &[u8], size: i32) -> Result<Vec<u8>, AppError> {
    let mut img = image::load_from_memory(file_bytes)
        .map_err(|e| AppError::Internal(eyre!(e).wrap_err("Failed to decode image for resize")))?;

    let mut decoder = ImageReader::open(path)
        .map_err(|e| AppError::Internal(eyre!("Failed to open image for orientation: {}", e)))?
        .into_decoder()
        .map_err(|e| AppError::Internal(eyre!("Failed to init decoder: {}", e)))?;

    if let Ok(orientation) = decoder.orientation() {
        img.apply_orientation(orientation);
    }

    let (width, height) = (img.width(), img.height());
    let aspect_ratio = width as f32 / height as f32;
    let (dst_width, dst_height) = if width > height {
        (size as u32, (size as f32 / aspect_ratio) as u32)
    } else {
        ((size as f32 * aspect_ratio) as u32, size as u32)
    };

    let src_image = fr::images::Image::from_vec_u8(
        width,
        height,
        img.to_rgb8().into_raw(),
        fr::PixelType::U8x3,
    )
    .map_err(|e| AppError::Internal(eyre!("Resize source creation error: {e}")))?;

    let mut dst_image = fr::images::Image::new(dst_width, dst_height, fr::PixelType::U8x3);
    let mut resizer = fr::Resizer::new();
    resizer
        .resize(&src_image, &mut dst_image, None)
        .map_err(|e| AppError::Internal(eyre!("Resizing failed: {e}")))?;

    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 80);

    encoder
        .encode(
            dst_image.buffer(),
            dst_width,
            dst_height,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| AppError::Internal(eyre!(e).wrap_err("Failed to encode JPEG")))?;

    Ok(buffer)
}

pub async fn stream_video_file(
    pool: &PgPool,
    ingest_settings: &IngestSettings,
    user_id: i32,
    user_role: UserRole,
    media_item_id: &str,
    range_header: Option<Range>,
) -> Result<Response<Body>, AppError> {
    let item = cached_store()
        .get_rel_path_and_user_id_for_media_item(pool, media_item_id)
        .await?;

    // Confirm ownership or admin privilege
    if item.user_id != user_id && user_role != UserRole::Admin {
        return Err(AppError::Forbidden("Denied".to_owned()));
    }

    let media_path = ingest_settings.media_root.join(&item.relative_path);
    let mut file = File::open(&media_path)
        .await
        .map_err(|_| AppError::Internal(eyre!("Failed to open video file")))?;
    let metadata = file
        .metadata()
        .await
        .map_err(|e| eyre!("Can't get file metadata {e}"))?;
    let file_size = metadata.len();

    let mut start = 0;
    let mut end = file_size - 1;
    let mut is_partial = false;

    if let Some(range) = range_header
        && let Some((start_bound, end_bound)) = range.satisfiable_ranges(file_size).next()
    {
        is_partial = true;

        start = match start_bound {
            Bound::Included(n) => n,
            Bound::Excluded(n) => n + 1,
            Bound::Unbounded => 0,
        };

        end = match end_bound {
            Bound::Included(n) => n,
            Bound::Excluded(n) => n - 1,
            Bound::Unbounded => file_size - 1,
        };
    }

    // Safety check for bounds
    if start > end || start >= file_size {
        return Err(AppError::RangeNotSatisfiable {
            message: "The requested byte range is unsatisfiable.".to_string(),
            file_size,
        });
    }

    let content_length = end - start + 1;

    file.seek(std::io::SeekFrom::Start(start))
        .await
        .map_err(|e| eyre!("Can't seek in video file {e}"))?;

    // streaming chunks
    let reader = file.take(content_length);
    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let mime_type = mime_guess::from_path(&media_path).first_or_octet_stream();

    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, mime_type.as_ref())
        .header(header::ACCEPT_RANGES, "bytes")
        .header(
            header::CACHE_CONTROL,
            "private, max-age=31536000, immutable",
        );

    if is_partial {
        response = response.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{file_size}"),
        );
    } else {
        response = response.status(StatusCode::OK);
    }

    Ok(response
        .header(header::CONTENT_LENGTH, content_length)
        .body(body)
        .map_err(|e| eyre!("Can't create video stream response {e}"))?)
}

async fn compute_updated_timestamps(
    executor: impl Executor<'_, Database = Postgres>,
    media_item_id: &str,
    new_local: Option<NaiveDateTime>,
    new_offset: &UpdateField<i32>,
) -> Result<(UpdateField<DateTime<Utc>>, Option<DateTime<Utc>>), AppError> {
    let is_updating_local = new_local.is_some();
    let is_updating_offset = !matches!(new_offset, UpdateField::Ignore);
    // If neither is changing, we don't need to update these derived fields
    if !is_updating_local && !is_updating_offset {
        return Ok((UpdateField::Ignore, None));
    }

    let current = sqlx::query!(
        "SELECT taken_at_local, timezone_offset_seconds FROM media_item WHERE id = $1",
        media_item_id
    )
    .fetch_one(executor)
    .await?;

    // Determine the values for this calculation
    let local_dt = new_local.unwrap_or(current.taken_at_local);
    let offset_secs = match new_offset {
        UpdateField::Value(v) => Some(*v),
        UpdateField::SetNull => None,
        UpdateField::Ignore => current.timezone_offset_seconds,
    };

    let taken_at_utc = offset_secs
        .and_then(FixedOffset::east_opt)
        .and_then(|offset| {
            offset
                .from_local_datetime(&local_dt)
                .earliest()
                .map(|dt| dt.with_timezone(&Utc))
        });

    let sort_timestamp = Some(with_fallback_timezone(taken_at_utc, &local_dt));
    let utc_field = taken_at_utc.map_or(UpdateField::SetNull, UpdateField::Value);

    Ok((utc_field, sort_timestamp))
}

pub async fn reprocess_media_item(
    pool: &PgPool,
    settings: &IngestSettings,
    media_item_id: &str,
    user_id: i32,
    relative_path: &str,
    file_path: &Path,
) -> Result<(), AppError> {
    let analyzer = media_analyzer::MediaAnalyzer::builder()
        .build()
        .await
        .map_err(|e| AppError::Internal(eyre!("Failed to build MediaAnalyzer: {}", e)))?;

    let media_info = analyzer
        .analyze_media(file_path)
        .await
        .map_err(|e| AppError::Internal(eyre!("Failed to analyze media {:?}: {}", file_path, e)))?;

    let create_item: CreateFullMediaItem = media_info.into();

    let mut tx = pool.begin().await?;

    // Delete visual_analysis records
    sqlx::query!(
        "DELETE FROM visual_analysis WHERE media_item_id = $1",
        media_item_id
    )
    .execute(&mut *tx)
    .await?;

    // Update main item and metadata child tables
    MediaItemStore::update_full(&mut tx, media_item_id, false, &create_item).await?;

    tx.commit().await?;

    // Enqueue worker reprocessing jobs
    enqueue_job::<()>(pool, settings, JobType::IngestThumbnails)
        .relative_path(relative_path)
        .user_id(user_id)
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::IngestAnalysis)
        .relative_path(relative_path)
        .user_id(user_id)
        .call()
        .await?;
    enqueue_job::<()>(pool, settings, JobType::IngestLlm)
        .relative_path(relative_path)
        .user_id(user_id)
        .call()
        .await?;

    Ok(())
}

pub async fn update_media_item(
    pool: &PgPool,
    settings: &IngestSettings,
    media_item_id: &str,
    user_id: i32,
    payload: &UpdateMediaItemRequest,
) -> Result<UpdateMediaItemResponse, AppError> {
    let mut current_id = media_item_id.to_owned();

    let media_user_id = MediaItemStore::find_user_by_id(pool, media_item_id)
        .await?
        .ok_or_else(|| AppError::NotFound(media_item_id.to_owned()))?;

    if media_user_id != user_id {
        return Err(AppError::Forbidden("Denied".to_owned()));
    }

    let UpdateMediaItemRequest {
        taken_at_local,
        user_caption,
        use_panorama_viewer,
        timezone_offset_seconds,
        orientation,
    } = payload;

    // Validate timezone offset if provided
    if let UpdateField::Value(v) = timezone_offset_seconds
        && FixedOffset::east_opt(*v).is_none()
    {
        return Err(AppError::BadRequest(format!(
            "Invalid timezone offset: {}",
            *v
        )));
    }

    // 1. Process EXIF Orientation update if provided
    if let Some(new_orientation) = orientation {
        let relative_path = MediaItemStore::find_relative_path_by_id(pool, media_item_id)
            .await?
            .ok_or_else(|| AppError::NotFound(media_item_id.to_owned()))?;

        let file_path = settings.media_root.join(&relative_path);

        write_exif_orientation(&file_path, *new_orientation)?;

        reprocess_media_item(
            pool,
            settings,
            media_item_id,
            user_id,
            &relative_path,
            &file_path,
        )
        .await?;

        let new_id = nice_id(app_state::constants().database.media_item_id_length);
        let mut tx = pool.begin().await?;
        MediaItemStore::change_media_item_id(&mut tx, media_item_id, &new_id).await?;
        tx.commit().await?;

        current_id = new_id;
    }

    // 2. Process caption / timestamp / panorama updates if provided
    let mut tx = pool.begin().await?;

    let taken_at_local_input = taken_at_local
        .as_ref()
        .map(|m| NaiveDateTime::parse_from_str(m, "%Y-%m-%dT%H:%M:%S"))
        .transpose()?;

    let (taken_at_utc, sort_timestamp) = compute_updated_timestamps(
        &mut *tx,
        &current_id,
        taken_at_local_input,
        timezone_offset_seconds,
    )
    .await?;

    MediaItemStore::update(
        &mut *tx,
        &current_id,
        UpdateMediaItemPayload {
            user_caption: user_caption.clone(),
            taken_at_local: taken_at_local_input,
            taken_at_utc,
            sort_timestamp,
            timezone_offset_seconds: timezone_offset_seconds.clone(),
            use_panorama_viewer: *use_panorama_viewer,
        },
    )
    .await?;

    tx.commit().await?;

    Ok(UpdateMediaItemResponse {
        media_item_id: current_id,
    })
}
