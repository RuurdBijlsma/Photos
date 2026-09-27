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

// ==========================================
// Configuration
// ==========================================
const INPUT_IMAGE_PATH: &str = "media_dir/rutenl/sunset.jpg";
const OUTPUT_DIR_PATH: &str = "test_out/format_comparison";
const TARGET_HEIGHTS: &[u32] = &[240, 480, 720, 1080, 1440];

// Quality parameters
const JXL_BUTTERAUGLI_DISTANCE: f32 = 1.5; // Lower is higher quality (1.0 = visually lossless)
const AVIF_QUALITY: f32 = 70.0;
const AVIF_SPEED: u8 = 6;
const WEBP_QUALITY: f32 = 80.0;
// ==========================================

fn encode_jxl(raw_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut encoder = encoder_builder()
        .lossless(false)
        .quality(JXL_BUTTERAUGLI_DISTANCE)
        .speed(EncoderSpeed::Falcon)
        .build()?;
    let result = encoder.encode::<u8, u8>(raw_pixels, width, height)?;
    Ok(result.data)
}

fn encode_avif(raw_rgba_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let rgba_pixels = raw_rgba_pixels.as_rgba();
    let img_ref = Img::new(rgba_pixels, width as usize, height as usize);
    let encoder = AvifEncoder::new()
        .with_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED)
        .with_alpha_quality(AVIF_QUALITY);
    let result = encoder.encode_rgba(img_ref)?;
    Ok(result.avif_file)
}

fn encode_webp(raw_rgba_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let encoder = webp::Encoder::from_rgba(raw_rgba_pixels, width, height);
    let webp_memory = encoder.encode(WEBP_QUALITY);
    Ok(webp_memory.to_vec())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let input_path = Path::new(INPUT_IMAGE_PATH);
    let output_dir = Path::new(OUTPUT_DIR_PATH);

    if !input_path.exists() {
        eprintln!("Input file not found at '{}'.", input_path.display());
        std::process::exit(1);
    }

    fs::create_dir_all(output_dir)?;

    // 1. Decode source image and handle orientation
    let decode_start = Instant::now();
    let reader = ImageReader::open(input_path)?.with_guessed_format()?;
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut dynamic_img = DynamicImage::from_decoder(decoder)?;
    dynamic_img.apply_orientation(orientation);

    let rgba_img = dynamic_img.into_rgba8();
    let (orig_w, orig_h) = rgba_img.dimensions();
    let orig_file_size = fs::metadata(input_path)?.len();
    let src_image = Image::from_vec_u8(orig_w, orig_h, rgba_img.into_raw(), PixelType::U8x4)?;

    println!("==========================================================================");
    println!("Image Codec Comparison (JXL vs AVIF vs WebP)");
    println!(
        "Source     : {} ({}x{}, {:.2} KB)",
        input_path.display(),
        orig_w,
        orig_h,
        orig_file_size as f64 / 1024.0
    );
    println!("Output Dir : {}", output_dir.display());
    println!("Decoded in : {:?}", decode_start.elapsed());
    println!("==========================================================================");
    println!(
        "{:<6} | {:<8} | {:<12} | {:<12} | {:<12}",
        "Format", "Height", "Dimensions", "Size (KB)", "Encode Time"
    );
    println!("{:-<60}", "");

    for &target_h in TARGET_HEIGHTS {
        let mut target_w = ((u64::from(orig_w) * u64::from(target_h)) / u64::from(orig_h)) as u32;
        if target_w % 2 != 0 {
            target_w += 1;
        }

        // Downscale to target resolution
        let mut dst_img = Image::new(target_w, target_h, PixelType::U8x4);
        let mut resizer = Resizer::new();
        resizer.resize(&src_image, &mut dst_img, None)?;
        let raw_rgba = dst_img.buffer();

        // --- 1. JXL ---
        let t0 = Instant::now();
        let jxl_bytes = encode_jxl(raw_rgba, target_w, target_h)?;
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

        // --- 2. AVIF ---
        let t0 = Instant::now();
        let avif_bytes = encode_avif(raw_rgba, target_w, target_h)?;
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

        // --- 3. WebP ---
        let t0 = Instant::now();
        let webp_bytes = encode_webp(raw_rgba, target_w, target_h)?;
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

    println!("All image files saved to: {}", output_dir.display());

    Ok(())
}