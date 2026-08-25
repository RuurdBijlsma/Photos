use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::Result;
use common_services::media_mount::{MediaMountStatus, media_mount_status};
use sqlx::PgPool;
use std::path::{Path, PathBuf};

/// If the source file is missing, cancel only when the media folder still looks mounted.
pub async fn result_if_source_missing(
    pool: &PgPool,
    media_root: &Path,
    file_path: &Path,
) -> Result<Option<JobResult>> {
    if file_path.exists() {
        return Ok(None);
    }
    match media_mount_status(pool, media_root).await? {
        MediaMountStatus::Available => Ok(Some(JobResult::Cancelled)),
        MediaMountStatus::Unavailable { reason } => Ok(Some(JobResult::StorageUnavailable(reason))),
    }
}

/// Returns `StorageUnavailable` when the media folder looks unmounted.
pub async fn require_media_mounted(pool: &PgPool, media_root: &Path) -> Result<Option<JobResult>> {
    match media_mount_status(pool, media_root).await? {
        MediaMountStatus::Available => Ok(None),
        MediaMountStatus::Unavailable { reason } => Ok(Some(JobResult::StorageUnavailable(reason))),
    }
}

/// Determines which thumbnail files should be sent to the ML analyzer.
#[must_use]
pub fn get_images_to_analyze(
    context: &WorkerContext,
    file_path: &Path,
    media_item_id: &str,
    percentages: &[u64],
    override_image_size: Option<u64>,
) -> Vec<(i32, PathBuf)> {
    let thumbnail_root = &context.settings.ingest.thumbnails_root;
    let thumb_dir = thumbnail_root.join(media_item_id);

    if context.settings.ingest.is_photo_file(file_path) {
        let analyze_image_size =
            override_image_size.unwrap_or(context.settings.ingest.analyzer.analyze_image_size);
        vec![(0, thumb_dir.join(format!("{analyze_image_size}p.avif")))]
    } else {
        percentages
            .iter()
            .map(|p| {
                (
                    i32::try_from(*p).expect("Percentage should fit in i32"),
                    thumb_dir.join(format!("{p}_percent.avif")),
                )
            })
            .collect()
    }
}
