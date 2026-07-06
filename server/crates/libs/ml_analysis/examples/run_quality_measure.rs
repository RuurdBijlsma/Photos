use image::{DynamicImage, imageops};
use ml_analysis::get_quality_measurement_from_image;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

#[allow(clippy::print_literal, clippy::too_many_lines)]
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let target_dir = Path::new("C:/Users/Ruurd/Pictures/small_media_dir");
    let output_base = Path::new("quality_test_output");

    let categories = [
        "weighted",
        "blurriness",
        "noisyness",
        "exposure",
        "accidentallness",
    ];

    println!(
        "Ensuring output directories exist under: {}",
        output_base.display()
    );
    for cat in &categories {
        let cat_dir = output_base.join(cat);
        fs::create_dir_all(&cat_dir)?;
    }

    println!("Scanning directory: {}", target_dir.display());
    let scan_start = Instant::now();

    // 1. Gather all file paths sequentially (fast metadata-only operation)
    let mut paths = Vec::new();
    for entry in WalkDir::new(target_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && (ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"))
        {
            paths.push(path.to_path_buf());
        }
    }

    println!("Found {} images. Processing in parallel...", paths.len());

    // 2. Process images in parallel using rayon
    let mut results: Vec<(PathBuf, common_types::ml_analysis::QualityMeasurement)> = paths
        .into_par_iter()
        .filter_map(|path| {
            let now = Instant::now();
            let file_name = path.file_name()?.to_string_lossy().to_string();

            // Decode image exactly once
            let img = match image::ImageReader::open(&path).and_then(|r| {
                r.decode().map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
                })
            }) {
                Ok(img) => img,
                Err(err) => {
                    eprintln!("Failed to decode image {}: {:?}", path.display(), err);
                    return None;
                }
            };

            // Downsizing first speeds up computation significantly on larger images.
            // Imageproc filters (like 5x5 median filters) are CPU intensive.
            // Performing analysis on a 2000px bounds image provides highly comparable
            // metrics with a fraction of the CPU and memory footprint.
            let resized = resize_if_large(img, 2000);

            // Compute quality measurement on the single decoded/resized image
            let quality_data = get_quality_measurement_from_image(&resized);

            // Save the resized image once (this handles the slow JPEG compression work)
            let weighted_name = format!("{:06.2}___{}", quality_data.weighted_score, file_name);
            let weighted_path = output_base.join("weighted").join(&weighted_name);

            if let Err(err) = resized.save(&weighted_path) {
                eprintln!("Failed to save output to weighted folder: {err:?}");
                return None;
            }

            // Copy the output file to the remaining folders (skips re-encoding JPEG 4 more times)
            let blur_name = format!("{:05.3}___{}", quality_data.blurriness, file_name);
            let _ = fs::copy(
                &weighted_path,
                output_base.join("blurriness").join(blur_name),
            );

            let noise_name = format!("{:05.3}___{}", quality_data.noisiness, file_name);
            let _ = fs::copy(
                &weighted_path,
                output_base.join("noisyness").join(noise_name),
            );

            let exposure_name = format!("{:05.3}___{}", quality_data.exposure, file_name);
            let _ = fs::copy(
                &weighted_path,
                output_base.join("exposure").join(exposure_name),
            );

            let accidental_name = format!("{:05.3}___{}", quality_data.accidentalness, file_name);
            let _ = fs::copy(
                &weighted_path,
                output_base.join("accidentallness").join(accidental_name),
            );

            println!("Analyzed {} in {:?}", file_name, now.elapsed());
            Some((path, quality_data))
        })
        .collect();

    println!(
        "\nFinished scanning {} images in {:?}",
        results.len(),
        scan_start.elapsed()
    );

    // Sort by weighted_score in ascending order (worst quality first)
    results.sort_by(|a, b| {
        a.1.weighted_score
            .partial_cmp(&b.1.weighted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Print sorted results
    println!(
        "\n====================================================================================================="
    );
    println!(
        "                           QUALITY ANALYSIS RESULTS (Worst to Best)                                  "
    );
    println!(
        "====================================================================================================="
    );
    println!(
        "{:<6} | {:<14} | {:<10} | {:<9} | {:<8} | {}",
        "Score", "Accidentalness", "Blurriness", "Noisiness", "Exposure", "File Path"
    );
    println!(
        "-----------------------------------------------------------------------------------------------------"
    );

    for (path, q) in &results {
        println!(
            "{:<6.1} | {:<14.3} | {:<10.3} | {:<9.3} | {:<8.3} | {}",
            q.weighted_score,
            q.accidentalness,
            q.blurriness,
            q.noisiness,
            q.exposure,
            path.display()
        );
    }
    println!(
        "====================================================================================================="
    );

    Ok(())
}

fn resize_if_large(img: DynamicImage, max_dim: u32) -> DynamicImage {
    if img.width() > max_dim || img.height() > max_dim {
        img.resize(max_dim, max_dim, imageops::FilterType::CatmullRom)
    } else {
        img
    }
}
