#![deny(clippy::unwrap_used)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

//! # Thumbnail Generation Crate
//!
//! A Rust crate for generating a variety of thumbnails for photos and videos.
//!
//! This library provides a flexible and efficient way to create thumbnails using either
//! native Rust image processing for formats like AVIF or by leveraging the power of `FFmpeg`
//! for a wide range of media types.
//!
//! ## Features
//!
//! - **Photo Thumbnails**: Generate high-quality, multi-sized thumbnails from source images.
//!   - Supports fast, native AVIF encoding.
//!   - Can fall back to `FFmpeg` for other formats.
//!   - Automatic EXIF orientation correction.
//! - **Video Thumbnails**: Create a comprehensive set of thumbnails from videos.
//!   - Generate still frames at specific timestamps or percentages.
//!   - Create transcoded video previews in different resolutions (e.g., 480p, 720p `WebM`).
//! - **Efficient Processing**:
//!   - Uses a single `FFmpeg` command to generate all required outputs simultaneously, minimizing process overhead.
//!   - Leverages parallel processing for native image resizing.
//! - **Flexible Configuration**: A comprehensive `ThumbOptions` struct allows for detailed control over the output.
//!
//! ## Entry Points
//!
//! The two main entry points for this crate are:
//!
//! - `generate_thumbnails`: The primary function to generate all configured thumbnails for a given file.
//! - `thumbs_exist`: A utility function to check if all expected thumbnails for a file already exist.

pub mod ffmpeg;
mod motion_photo;
mod pano;
mod photo;
mod utils;
mod video;

pub use crate::motion_photo::generate_motion_thumbs;
use crate::pano::generate_pano_thumbs;
use app_state::IngestSettings;
use color_eyre::Result;
use color_eyre::eyre::bail;
use std::path::Path;
use temp_dir::TempDir;
use tracing::warn;
pub use utils::{copy_dir_contents, link_or_copy_dir_contents};

#[derive(Clone, Debug, Default)]
pub struct ThumbnailConfig {
    pub generate_panorama_tiles: bool,
    pub extract_motion_photo: bool,
    pub orientation: i32,
}

#[derive(Clone, Debug, Default)]
pub struct ThumbnailOutput {
    pub already_exists: bool,
    pub pano_success: bool,
    pub motion_success: bool,
}

/// Generates thumbnails for a given media file (image or video) based on the provided configuration.
///
/// This function detects the file type and calls the appropriate thumbnail generation logic.
/// Generated files are created in a temporary directory and then moved to their final destination.
///
/// # Arguments
///
/// * `file` - The path to the source image or video file.
/// * `out_folder` - Where to output the thumbnail files.
/// * `config` - An `ThumbOptions` struct detailing what thumbnails to generate.
/// * `orientation` - The EXIF orientation value (1-8) for photos.
///
/// # Errors
///
/// Returns an error if paths are invalid, `FFmpeg` commands fail, or file I/O operations fail.
pub async fn generate_thumbnails(
    ingestion: &IngestSettings,
    file: &Path,
    thumb_sub_folder: &Path,
    pano_sub_folder: &Path,
    config: ThumbnailConfig,
) -> Result<ThumbnailOutput> {
    let Some(sub_folder_name) = thumb_sub_folder
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
    else {
        bail!("Thumbnail subfolder must have a name");
    };

    // todo: thumbs_exist should include panorama and motion photo video
    if !ingestion.thumbnails.recreate_if_exists
        && ingestion.thumbs_exist(file, &sub_folder_name, Some(pano_sub_folder), true)?
    {
        return Ok(ThumbnailOutput {
            already_exists: true,
            motion_success: false,
            pano_success: false,
        });
    }

    let temp_dir = TempDir::new()?;
    let temp_out_dir = temp_dir.path();
    let mut output = ThumbnailOutput {
        pano_success: true,
        already_exists: false,
        motion_success: true,
    };

    if ingestion.is_photo_file(file) {
        if ingestion.thumbnails.thumbnail_extension == "avif" {
            photo::generate_native_photo_thumbnails(
                file,
                temp_out_dir,
                &ingestion.thumbnails,
                config.orientation,
            )?;
        } else {
            photo::generate_ffmpeg_photo_thumbnails(file, temp_out_dir, &ingestion.thumbnails)
                .await?;
        }

        if config.generate_panorama_tiles
            && let Err(e) = generate_pano_thumbs(file, pano_sub_folder)
        {
            warn!("Failed to generate panorama for {}, {}", file.display(), e);
            output.pano_success = false;
        }
        if config.extract_motion_photo
            && let Err(e) = generate_motion_thumbs(file, temp_out_dir)
        {
            warn!(
                "Failed to generate motion photo video for {}, {}",
                file.display(),
                e
            );
            output.motion_success = false;
        }
    } else if ingestion.is_video_file(file) {
        video::generate_video_thumbnails(file, temp_out_dir, &ingestion.thumbnails).await?;
    }

    utils::move_dir_contents(temp_out_dir, thumb_sub_folder).await?;
    temp_dir.cleanup()?;

    Ok(output)
}
