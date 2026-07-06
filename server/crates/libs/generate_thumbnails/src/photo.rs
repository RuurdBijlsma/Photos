use crate::ffmpeg::FfmpegCommand;
use app_state::ThumbnailSettings;
use color_eyre::eyre::{Result, eyre};
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, Resizer};
use image::metadata::Orientation;
use image::{DynamicImage, ImageReader};
use imgref::Img;
use libheif_rs::{HeifContext, LibHeif};
use ravif::Encoder;
use rayon::prelude::*;
use rgb::FromSlice;
use std::fs;
use std::path::Path;

pub fn open_dynamic_image(input_path: &Path, orientation: i32) -> Result<DynamicImage> {
    let extension = input_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    let is_heif = matches!(
        extension.as_deref(),
        Some("heic" | "heif" | "heics" | "heifs")
    );

    let img = if is_heif {
        let input_str = input_path
            .to_str()
            .ok_or_else(|| eyre!("Input path is not valid UTF-8"))?;
        let lib_heif = LibHeif::new();
        let ctx = HeifContext::read_from_file(input_str)?;
        let handle = ctx.primary_image_handle()?;
        let decoded = lib_heif.decode(
            &handle,
            libheif_rs::ColorSpace::Rgb(libheif_rs::RgbChroma::Rgb),
            None,
        )?;
        let width = decoded.width();
        let height = decoded.height();
        if height == 0 {
            return Err(eyre!("Image height cannot be zero"));
        }
        let planes = decoded.planes();
        let interleaved_plane = planes
            .interleaved
            .ok_or_else(|| eyre!("Could not retrieve interleaved RGB plane from decoded image"))?;

        let stride = interleaved_plane.stride;
        let data = interleaved_plane.data;

        // Re-pack stride padded rows into a tightly packed RGB vector for the image crate
        let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            let row_start = y as usize * stride;
            let row_end = row_start + (width as usize * 3);
            if row_end <= data.len() {
                rgb_data.extend_from_slice(&data[row_start..row_end]);
            } else {
                return Err(eyre!("Interleaved plane data is smaller than expected"));
            }
        }

        let rgb_img = image::RgbImage::from_raw(width, height, rgb_data)
            .ok_or_else(|| eyre!("Failed to create RgbImage from decoded pixel data"))?;
        DynamicImage::ImageRgb8(rgb_img)
    } else {
        let mut img = ImageReader::open(input_path)?
            .with_guessed_format()?
            .decode()?;
        // Correct the orientation based on the EXIF data.
        let image_orientation = match orientation {
            2 => Orientation::FlipHorizontal,
            3 => Orientation::Rotate180,
            4 => Orientation::FlipVertical,
            5 => Orientation::Rotate90FlipH,
            6 => Orientation::Rotate90,
            7 => Orientation::Rotate270FlipH,
            8 => Orientation::Rotate270,
            _ => Orientation::NoTransforms,
        };
        img.apply_orientation(image_orientation);
        img
    };

    Ok(img)
}

/// Generates photo thumbnails using a native Rust image processing library.
/// This is optimized for AVIF output and supports EXIF orientation correction.
pub fn generate_native_photo_thumbnails(
    input_path: &Path,
    output_dir: &Path,
    config: &ThumbnailSettings,
    orientation: i32,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    if config.heights.is_empty() {
        return Ok(());
    }

    let img = open_dynamic_image(input_path, orientation)?;

    let src_img = img.into_rgba8();
    let (orig_w, orig_h) = src_img.dimensions();

    let src_image = Image::from_vec_u8(orig_w, orig_h, src_img.into_raw(), PixelType::U8x4)?;

    config
        .heights
        .par_iter()
        .try_for_each(|&target_h| -> Result<()> {
            let mut target_w = ((u64::from(orig_w) * target_h) / u64::from(orig_h)) as u32;
            if target_w > 0 && !target_w.is_multiple_of(2) {
                target_w += 1;
            }
            let mut dst_img = Image::new(target_w, target_h as u32, PixelType::U8x4);
            let mut r = Resizer::new();
            r.resize(&src_image, &mut dst_img, None)?;
            let raw_pixels = dst_img.buffer();
            let rgba_pixels = raw_pixels.as_rgba();
            let img_ref = Img::new(rgba_pixels, target_w as usize, target_h as usize);
            let encoder = Encoder::new()
                .with_quality(config.avif_options.quality)
                .with_speed(config.avif_options.speed)
                .with_alpha_quality(config.avif_options.alpha_quality);
            let avif_data = encoder.encode_rgba(img_ref)?;
            fs::write(
                output_dir.join(format!("{target_h}p.avif")),
                avif_data.avif_file,
            )?;

            Ok(())
        })?;

    Ok(())
}

/// Generates photo thumbnails using `FFmpeg`.
pub async fn generate_ffmpeg_photo_thumbnails(
    input: &Path,
    output_dir: &Path,
    config: &ThumbnailSettings,
) -> Result<()> {
    if config.heights.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(output_dir)?;

    let mut cmd = FfmpegCommand::new(input);
    let input_stream = "[0:v]";
    let output_streams = cmd.add_split(input_stream, config.heights.len());

    for (i, &h) in config.heights.iter().enumerate() {
        // -2 logic ensures even width
        let scaled_stream = cmd.add_scale(&output_streams[i], -2, h as i32);
        let out_path = output_dir.join(format!("{h}p.{}", config.thumbnail_extension));
        cmd.map_still_output(&scaled_stream, &out_path);
    }

    cmd.run().await
}
