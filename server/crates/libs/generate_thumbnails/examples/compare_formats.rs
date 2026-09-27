use app_state::load_app_settings;
use color_eyre::Result;
use fast_image_resize::images::Image;
use fast_image_resize::{PixelType, Resizer};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader};
use imgref::Img;
use jpegxl_rs::encode::EncoderSpeed;
use jpegxl_rs::encoder_builder;
use ravif::Encoder as AvifEncoder;
use rgb::FromSlice;
use std::fs;
use std::path::Path;
use std::time::Instant;

// =========================================================================
// Paths & Codec Target Matching
// =========================================================================
const INPUT_IMAGE_PATH: &str = "media_dir/rutenl/sunset.jpg";
const OUTPUT_DIR_PATH: &str = "test_out/format_comparison";

// JXL distance 1.1 closely matches AVIF Q80 in visual fidelity and file size.
// (Butteraugli scale: 0.0 = lossless, 1.0 = visually lossless threshold)
const JXL_BUTTERAUGLI_DISTANCE: f32 = 1.1;
const JXL_SPEED: EncoderSpeed = EncoderSpeed::Falcon;

// WebP 82 matches AVIF Q80 perceptual quality
const WEBP_QUALITY: f32 = 82.0;
// =========================================================================

fn encode_jxl(raw_rgb_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut encoder = encoder_builder()
        .lossless(false)
        .quality(JXL_BUTTERAUGLI_DISTANCE)
        .speed(JXL_SPEED)
        .build()?;
    let result = encoder.encode::<u8, u8>(raw_rgb_pixels, width, height)?;
    Ok(result.data)
}

fn encode_avif(
    raw_rgba_pixels: &[u8],
    width: u32,
    height: u32,
    quality: f32,
    alpha_quality: f32,
    speed: u8,
) -> Result<Vec<u8>> {
    let rgba_pixels = raw_rgba_pixels.as_rgba();
    let img_ref = Img::new(rgba_pixels, width as usize, height as usize);
    let encoder = AvifEncoder::new()
        .with_quality(quality)
        .with_speed(speed)
        .with_alpha_quality(alpha_quality);
    let result = encoder.encode_rgba(img_ref)?;
    Ok(result.avif_file)
}

fn encode_webp(raw_rgb_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let encoder = webp::Encoder::from_rgb(raw_rgb_pixels, width, height);
    let webp_memory = encoder.encode(WEBP_QUALITY);
    Ok(webp_memory.to_vec())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let thumb_cfg = &settings.ingest.thumbnails;

    let input_path = Path::new(INPUT_IMAGE_PATH);
    let output_dir = Path::new(OUTPUT_DIR_PATH);

    if !input_path.exists() {
        eprintln!("Input file not found at '{}'.", input_path.display());
        std::process::exit(1);
    }

    fs::create_dir_all(output_dir)?;

    // 1. Decode source image and handle EXIF orientation
    let decode_start = Instant::now();
    let reader = ImageReader::open(input_path)?.with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut dynamic_img = DynamicImage::from_decoder(decoder)?;
    dynamic_img.apply_orientation(orientation);

    let (orig_w, orig_h) = (dynamic_img.width(), dynamic_img.height());
    let orig_file_size = fs::metadata(input_path)?.len();

    // Prepare separate source buffers:
    // - RGB for JXL and WebP
    // - RGBA for AVIF (ravif)
    let rgb_img = dynamic_img.to_rgb8();
    let src_image_rgb =
        Image::from_vec_u8(orig_w, orig_h, rgb_img.into_raw(), PixelType::U8x3)?;

    let rgba_img = dynamic_img.to_rgba8();
    let src_image_rgba =
        Image::from_vec_u8(orig_w, orig_h, rgba_img.into_raw(), PixelType::U8x4)?;

    println!("==========================================================================");
    println!("Codec Comparison (Using settings.yaml config)");
    println!(
        "Source     : {} ({}x{}, {:.2} KB)",
        input_path.display(),
        orig_w,
        orig_h,
        orig_file_size as f64 / 1024.0
    );
    println!("Output Dir : {}", output_dir.display());
    println!(
        "Settings   : AVIF [q: {}, speed: {}], JXL [distance: {:.1}, speed: {:?}], WebP [q: {:.1}]",
        thumb_cfg.avif_options.quality,
        thumb_cfg.avif_options.speed,
        JXL_BUTTERAUGLI_DISTANCE,
        JXL_SPEED,
        WEBP_QUALITY
    );
    println!("Decoded in : {:?}", decode_start.elapsed());
    println!("==========================================================================");
    println!(
        "{:<6} | {:<8} | {:<12} | {:<12} | {:<12}",
        "Format", "Height", "Dimensions", "Size (KB)", "Encode Time"
    );
    println!("{:-<60}", "");

    let mut resizer = Resizer::new();

    for &target_h_u64 in &thumb_cfg.heights {
        let target_h = target_h_u64 as u32;
        let mut target_w = ((u64::from(orig_w) * target_h_u64) / u64::from(orig_h)) as u32;
        if target_w % 2 != 0 {
            target_w += 1;
        }

        // Downscale to target RGB and RGBA buffers
        let mut dst_rgb = Image::new(target_w, target_h, PixelType::U8x3);
        resizer.resize(&src_image_rgb, &mut dst_rgb, None)?;
        let raw_rgb = dst_rgb.buffer();

        let mut dst_rgba = Image::new(target_w, target_h, PixelType::U8x4);
        resizer.resize(&src_image_rgba, &mut dst_rgba, None)?;
        let raw_rgba = dst_rgba.buffer();

        // 1. AVIF (Q80, Speed 4)
        let t0 = Instant::now();
        let avif_bytes = encode_avif(
            raw_rgba,
            target_w,
            target_h,
            thumb_cfg.avif_options.quality as f32,
            thumb_cfg.avif_options.alpha_quality as f32,
            thumb_cfg.avif_options.speed,
        )?;
        let avif_time = t0.elapsed().as_secs_f64() * 1000.0;
        let avif_path = output_dir.join(format!("{target_h}p_avif.avif"));
        fs::write(&avif_path, &avif_bytes)?;
        println!(
            "{:<6} | {:<8} | {:<12} | {:<12.2} | {:<10.2} ms",
            "AVIF",
            format!("{target_h}p"),
            format!("{target_w}x{target_h}"),
            avif_bytes.len() as f64 / 1024.0,
            avif_time
        );

        // 2. JXL (Butteraugli 1.1, Falcon)
        let t0 = Instant::now();
        let jxl_bytes = encode_jxl(raw_rgb, target_w, target_h)?;
        let jxl_time = t0.elapsed().as_secs_f64() * 1000.0;
        let jxl_path = output_dir.join(format!("{target_h}p_jxl.jxl"));
        fs::write(&jxl_path, &jxl_bytes)?;
        println!(
            "{:<6} | {:<8} | {:<12} | {:<12.2} | {:<10.2} ms",
            "JXL",
            format!("{target_h}p"),
            format!("{target_w}x{target_h}"),
            jxl_bytes.len() as f64 / 1024.0,
            jxl_time
        );

        // 3. WebP (Q82)
        let t0 = Instant::now();
        let webp_bytes = encode_webp(raw_rgb, target_w, target_h)?;
        let webp_time = t0.elapsed().as_secs_f64() * 1000.0;
        let webp_path = output_dir.join(format!("{target_h}p_webp.webp"));
        fs::write(&webp_path, &webp_bytes)?;
        println!(
            "{:<6} | {:<8} | {:<12} | {:<12.2} | {:<10.2} ms",
            "WebP",
            format!("{target_h}p"),
            format!("{target_w}x{target_h}"),
            webp_bytes.len() as f64 / 1024.0,
            webp_time
        );

        println!("{:-<60}", "");
    }

    println!("Finished! Saved images to: {}", output_dir.display());

    Ok(())
}