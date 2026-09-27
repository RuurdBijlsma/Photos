use color_eyre::Result;
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, Resizer};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};
use jpegxl_rs::encode::EncoderSpeed;
use jpegxl_rs::encoder_builder;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const TARGET_HEIGHTS: &[u32] = &[240, 480, 720, 1080, 1440];

fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();
    let default_input = "media_dir/rutenl/sunset.jpg".to_string();
    let default_output = "test_out/jxl_thumbs".to_string();

    let input_path_str = args.get(1).unwrap_or(&default_input);
    let output_dir_str = args.get(2).unwrap_or(&default_output);

    let input_path = Path::new(input_path_str);
    let output_dir = PathBuf::from(output_dir_str);

    if !input_path.exists() {
        eprintln!(
            "Input file not found at '{}'.\n\
             Usage:\n  \
             cargo run --example jxl -- <path_to_image> [output_dir]",
            input_path.display()
        );
        std::process::exit(1);
    }

    fs::create_dir_all(&output_dir)?;

    println!("====================================================");
    println!("JPEG XL Thumbnail Generation POC");
    println!("Input file : {}", input_path.display());
    println!("Output dir : {}", output_dir.display());
    println!("====================================================");

    let total_start = Instant::now();

    // 1. Decode source image and handle EXIF orientation
    let decode_start = Instant::now();
    let reader = ImageReader::open(input_path)?.with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut dynamic_img = DynamicImage::from_decoder(decoder)?;
    dynamic_img.apply_orientation(orientation);

    // Detect if image has transparency
    let has_alpha = dynamic_img.color().has_alpha();
    let (orig_w, orig_h) = (dynamic_img.width(), dynamic_img.height());
    let orig_file_size = fs::metadata(input_path)?.len();

    println!("====================================================");
    println!("JPEG XL Thumbnail Generation POC");
    println!("Input file   : {}", input_path.display());
    println!("Output dir   : {}", output_dir.display());
    println!("Dimensions   : {}x{}", orig_w, orig_h);
    println!("Has alpha    : {}", has_alpha);
    println!("Decoded in   : {:?}", decode_start.elapsed());
    println!("====================================================");

    // 2. Prepare source image buffer for fast_image_resize
    let pixel_type = if has_alpha {
        PixelType::U8x4
    } else {
        PixelType::U8x3
    };

    let raw_src_bytes = if has_alpha {
        dynamic_img.into_rgba8().into_raw()
    } else {
        dynamic_img.into_rgb8().into_raw()
    };

    let src_image = Image::from_vec_u8(orig_w, orig_h, raw_src_bytes, pixel_type)?;

    // 3. Generate thumbnails in parallel for each target height
    println!("\nGenerating JXL thumbnails for heights: {TARGET_HEIGHTS:?}");

    TARGET_HEIGHTS
        .par_iter()
        .try_for_each(|&target_h| -> Result<()> {
            let start = Instant::now();

            // Calculate scaled width preserving aspect ratio (rounded to even)
            let mut target_w =
                ((u64::from(orig_w) * u64::from(target_h)) / u64::from(orig_h)) as u32;
            if target_w > 0 && target_w % 2 != 0 {
                target_w += 1;
            }

            // Downscale buffer
            let mut dst_img = Image::new(target_w, target_h, pixel_type);
            let mut resizer = Resizer::new();
            resizer.resize(&src_image, &mut dst_img, None)?;

            // Configure JPEG XL encoder:
            // Butteraugli distance (0.0 = lossless, 1.0 = visually lossless, 1.5 = high quality)
            let mut encoder = encoder_builder()
                .lossless(false)
                .quality(1.5)
                .speed(EncoderSpeed::Falcon)
                .build()?;

            let raw_pixels = dst_img.buffer();
            let result = encoder.encode::<u8, u8>(raw_pixels, target_w, target_h)?;

            let out_file = output_dir.join(format!("{target_h}p.jxl"));
            fs::write(&out_file, &result.data)?;

            let size = result.data.len();
            println!(
                "  [{target_h}p] Generated {}x{} (alpha: {}) -> {} ({} bytes, {:.2} KB) in {:?}",
                target_w,
                target_h,
                has_alpha,
                out_file.file_name().unwrap().to_string_lossy(),
                size,
                size as f64 / 1024.0,
                start.elapsed()
            );

            Ok(())
        })?;

    println!("\n====================================================");
    println!("Original file size : {:.2} KB", orig_file_size as f64 / 1024.0);
    println!("Total elapsed time : {:?}", total_start.elapsed());
    println!("Thumbnails saved to: {}", output_dir.display());
    println!("====================================================");

    Ok(())
}