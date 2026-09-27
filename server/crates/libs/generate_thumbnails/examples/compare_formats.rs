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
use std::path::{Path, PathBuf};
use std::time::Instant;

const TARGET_HEIGHTS: &[u32] = &[240, 480, 720, 1080, 1440];

struct EncodedResult {
    format: &'static str,
    height: u32,
    width: u32,
    size_bytes: usize,
    duration_ms: f64,
    filename: String,
}

fn encode_jxl(raw_pixels: &[u8], width: u32, height: u32, quality: f32) -> Result<Vec<u8>> {
    let mut encoder = encoder_builder()
        .lossless(false)
        .quality(quality) // Butteraugli distance: 1.0 (visually lossless) to 1.5 (high quality)
        .speed(EncoderSpeed::Falcon)
        .build()?;
    let result = encoder.encode::<u8, u8>(raw_pixels, width, height)?;
    Ok(result.data)
}

fn encode_avif(raw_rgba_pixels: &[u8], width: u32, height: u32, quality: f32) -> Result<Vec<u8>> {
    let rgba_pixels = raw_rgba_pixels.as_rgba();
    let img_ref = Img::new(rgba_pixels, width as usize, height as usize);
    let encoder = AvifEncoder::new()
        .with_quality(quality)
        .with_speed(6)
        .with_alpha_quality(quality);
    let result = encoder.encode_rgba(img_ref)?;
    Ok(result.avif_file)
}

fn encode_webp(raw_rgba_pixels: &[u8], width: u32, height: u32, quality: f32) -> Result<Vec<u8>> {
    let encoder = webp::Encoder::from_rgba(raw_rgba_pixels, width, height);
    let webp_memory = encoder.encode(quality);
    Ok(webp_memory.to_vec())
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();
    let default_input = "media_dir/rutenl/sunset.jpg".to_string();
    let default_output = "test_out/format_comparison".to_string();

    let input_path_str = args.get(1).unwrap_or(&default_input);
    let output_dir_str = args.get(2).unwrap_or(&default_output);

    let input_path = Path::new(input_path_str);
    let output_dir = PathBuf::from(output_dir_str);

    if !input_path.exists() {
        eprintln!(
            "Input file not found at '{}'.\nUsage: cargo run --example compare_formats -- <image_path> [out_dir]",
            input_path.display()
        );
        std::process::exit(1);
    }

    fs::create_dir_all(&output_dir)?;

    // 1. Decode source image
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
    println!("Image Format Encoder Comparison (JXL vs AVIF vs WebP)");
    println!("Source: {} ({}x{}, {:.2} KB)", input_path.display(), orig_w, orig_h, orig_file_size as f64 / 1024.0);
    println!("Decoded in: {:?}", decode_start.elapsed());
    println!("==========================================================================");

    let mut results: Vec<EncodedResult> = Vec::new();

    for &target_h in TARGET_HEIGHTS {
        let mut target_w = ((u64::from(orig_w) * u64::from(target_h)) / u64::from(orig_h)) as u32;
        if target_w % 2 != 0 {
            target_w += 1;
        }

        // Downscale
        let mut dst_img = Image::new(target_w, target_h, PixelType::U8x4);
        let mut resizer = Resizer::new();
        resizer.resize(&src_image, &mut dst_img, None)?;
        let raw_rgba = dst_img.buffer();

        // 1. JXL (quality 1.5 Butteraugli distance)
        let t0 = Instant::now();
        let jxl_bytes = encode_jxl(raw_rgba, target_w, target_h, 1.5)?;
        let jxl_time = t0.elapsed().as_secs_f64() * 1000.0;
        let jxl_filename = format!("{target_h}p.jxl");
        fs::write(output_dir.join(&jxl_filename), &jxl_bytes)?;
        results.push(EncodedResult {
            format: "JXL",
            height: target_h,
            width: target_w,
            size_bytes: jxl_bytes.len(),
            duration_ms: jxl_time,
            filename: jxl_filename,
        });

        // 2. AVIF (quality 70.0)
        let t0 = Instant::now();
        let avif_bytes = encode_avif(raw_rgba, target_w, target_h, 70.0)?;
        let avif_time = t0.elapsed().as_secs_f64() * 1000.0;
        let avif_filename = format!("{target_h}p.avif");
        fs::write(output_dir.join(&avif_filename), &avif_bytes)?;
        results.push(EncodedResult {
            format: "AVIF",
            height: target_h,
            width: target_w,
            size_bytes: avif_bytes.len(),
            duration_ms: avif_time,
            filename: avif_filename,
        });

        // 3. WebP (quality 80.0)
        let t0 = Instant::now();
        let webp_bytes = encode_webp(raw_rgba, target_w, target_h, 80.0)?;
        let webp_time = t0.elapsed().as_secs_f64() * 1000.0;
        let webp_filename = format!("{target_h}p.webp");
        fs::write(output_dir.join(&webp_filename), &webp_bytes)?;
        results.push(EncodedResult {
            format: "WebP",
            height: target_h,
            width: target_w,
            size_bytes: webp_bytes.len(),
            duration_ms: webp_time,
            filename: webp_filename,
        });
    }

    // Print Results Table
    println!(
        "\n{:<6} | {:<8} | {:<12} | {:<12} | {:<12}",
        "Format", "Height", "Dimensions", "Size (KB)", "Encode Time"
    );
    println!("{:-<60}", "");
    for r in &results {
        println!(
            "{:<6} | {:<8} | {:<12} | {:<12.2} | {:<10.2} ms",
            r.format,
            format!("{}p", r.height),
            format!("{}x{}", r.width, r.height),
            r.size_bytes as f64 / 1024.0,
            r.duration_ms
        );
    }

    // Generate HTML visual inspection file
    generate_html_report(&output_dir, &results)?;
    println!("\nGenerated visual inspection report: {}", output_dir.join("index.html").display());

    Ok(())
}

fn generate_html_report(output_dir: &Path, results: &[EncodedResult]) -> Result<()> {
    let mut rows_html = String::new();

    for &h in TARGET_HEIGHTS {
        let jxl = results.iter().find(|r| r.height == h && r.format == "JXL").unwrap();
        let avif = results.iter().find(|r| r.height == h && r.format == "AVIF").unwrap();
        let webp = results.iter().find(|r| r.height == h && r.format == "WebP").unwrap();

        rows_html.push_str(&format!(
            r#"
            <div class="row">
                <h2>Resolution: {h}p ({w}x{h})</h2>
                <div class="cards">
                    <div class="card">
                        <h3>JXL (Butteraugli 1.5, Falcon)</h3>
                        <p>Size: <b>{jxl_kb:.2} KB</b> | Time: <b>{jxl_time:.2} ms</b></p>
                        <img src="{jxl_file}" alt="JXL {h}p" loading="lazy" />
                    </div>
                    <div class="card">
                        <h3>AVIF (Q70, Speed 6)</h3>
                        <p>Size: <b>{avif_kb:.2} KB</b> | Time: <b>{avif_time:.2} ms</b></p>
                        <img src="{avif_file}" alt="AVIF {h}p" loading="lazy" />
                    </div>
                    <div class="card">
                        <h3>WebP (Q80)</h3>
                        <p>Size: <b>{webp_kb:.2} KB</b> | Time: <b>{webp_time:.2} ms</b></p>
                        <img src="{webp_file}" alt="WebP {h}p" loading="lazy" />
                    </div>
                </div>
            </div>
            "#,
            h = h,
            w = jxl.width,
            jxl_kb = jxl.size_bytes as f64 / 1024.0,
            jxl_time = jxl.duration_ms,
            jxl_file = jxl.filename,
            avif_kb = avif.size_bytes as f64 / 1024.0,
            avif_time = avif.duration_ms,
            avif_file = avif.filename,
            webp_kb = webp.size_bytes as f64 / 1024.0,
            webp_time = webp.duration_ms,
            webp_file = webp.filename,
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Thumbnail Codec Comparison</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; background: #1a1a1a; color: #f0f0f0; margin: 20px; }}
        h1, h2 {{ text-align: center; }}
        .row {{ background: #262626; border-radius: 8px; padding: 16px; margin-bottom: 32px; }}
        .cards {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 16px; }}
        .card {{ background: #333; padding: 12px; border-radius: 6px; text-align: center; }}
        .card img {{ max-width: 100%; height: auto; border-radius: 4px; background: #000; }}
        .card p {{ font-size: 0.9em; color: #ccc; margin: 8px 0; }}
    </style>
</head>
<body>
    <h1>Thumbnail Quality & Size Comparison</h1>
    {rows_html}
</body>
</html>"#
    );

    fs::write(output_dir.join("index.html"), html)?;
    Ok(())
}