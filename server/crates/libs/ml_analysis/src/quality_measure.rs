use color_eyre::eyre::Result;
use common_types::ml_analysis::QualityMeasurement;
use image::{GrayImage, ImageBuffer, Luma};
use imageproc::filter::{laplacian_filter, median_filter};
use std::path::Path;

/// Analyzes an image to determine its overall quality score based on blurriness, noisiness, exposure, and accidentalness.
///
/// # Errors
///
/// This function will return an error if the image path is invalid or the image cannot be decoded.
/// Analyzes an image to determine its overall quality score based on blurriness, noisiness, exposure, and accidentalness.
pub fn get_quality_measurement(image_path: &Path) -> Result<QualityMeasurement> {
    let img = image::ImageReader::open(image_path)?.decode()?;
    Ok(get_quality_measurement_from_image(&img))
}

/// Measures quality from an already decoded image to avoid duplicate decoding.
#[must_use]
pub fn get_quality_measurement_from_image(img: &image::DynamicImage) -> QualityMeasurement {
    let gray_img = img.to_luma8();
    get_quality_measurement_from_luma(&gray_img)
}

/// Measures quality directly from a grayscale image.
#[must_use]
pub fn get_quality_measurement_from_luma(gray_img: &GrayImage) -> QualityMeasurement {
    let accidentalness = calculate_accidentalness(gray_img);
    let texture = calculate_texture(gray_img);
    let blurriness = calculate_blurriness(gray_img, texture);
    let noisiness = calculate_noise(gray_img, texture);
    let exposure = calculate_exposure(gray_img);

    // Scaling down the weighted score based on how confidently bad the photo is.
    let weighted_score = (1.0 - accidentalness).mul_add(
        0.3,
        exposure.mul_add(0.2, noisiness.mul_add(0.1, blurriness * 0.4)),
    ) * 100.0;

    QualityMeasurement {
        weighted_score,
        blurriness,
        noisiness,
        exposure,
        accidentalness,
    }
}

fn calculate_contrast(gray_img: &GrayImage) -> f64 {
    let n = f64::from(gray_img.width() * gray_img.height());
    if n == 0.0 {
        return 0.0;
    }

    let mean = gray_img.pixels().map(|p| f64::from(p[0])).sum::<f64>() / n;
    ((gray_img
        .pixels()
        .map(|p| (f64::from(p[0]) - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt())
        / 255.0
}

fn calculate_texture(gray_img: &GrayImage) -> f64 {
    let (w, h) = (gray_img.width() as usize, gray_img.height() as usize);
    let window = 5;
    let half = window / 2;

    let mut sum = vec![0f64; (w + 1) * (h + 1)];
    let mut sum_sq = vec![0f64; (w + 1) * (h + 1)];

    let idx = |x: usize, y: usize| y * (w + 1) + x;

    for y in 0..h {
        let mut row_sum = 0.0;
        let mut row_sum_sq = 0.0;
        for x in 0..w {
            let val = f64::from(gray_img.get_pixel(x as u32, y as u32)[0]);
            row_sum += val;
            row_sum_sq = val.mul_add(val, row_sum_sq);

            sum[idx(x + 1, y + 1)] = sum[idx(x + 1, y)] + row_sum;
            sum_sq[idx(x + 1, y + 1)] = sum_sq[idx(x + 1, y)] + row_sum_sq;
        }
    }

    let mut total_sd = 0.0;
    let mut count = 0.0;

    for y in half..(h - half) {
        for x in half..(w - half) {
            let (x0, y0) = (x - half, y - half);
            let (x1, y1) = (x + half + 1, y + half + 1);
            let n = (window * window) as f64;

            let sum_w = sum[idx(x1, y1)] + sum[idx(x0, y0)] - sum[idx(x1, y0)] - sum[idx(x0, y1)];
            let sum_sq_w = sum_sq[idx(x1, y1)] + sum_sq[idx(x0, y0)]
                - sum_sq[idx(x1, y0)]
                - sum_sq[idx(x0, y1)];

            let mean = sum_w / n;

            total_sd += mean.mul_add(-mean, sum_sq_w / n).max(0.0).sqrt();
            count += 1.0;
        }
    }

    (total_sd / count) / 128.0
}

/// Evaluates sharpness using a grid-based approach.
/// Divides the image into blocks, calculates local Laplacian variance, and analyzes the top 10%
/// sharpest blocks. This correctly handles photos with shallow depth-of-field (bokeh backgrounds).
fn calculate_blurriness(gray_img: &GrayImage, texture: f64) -> f64 {
    let lap = laplacian_filter(gray_img);
    let (width, height) = (lap.width(), lap.height());

    let grid_size = 10; // 10x10 grid of blocks
    let block_w = width / grid_size;
    let block_h = height / grid_size;

    if block_w < 4 || block_h < 4 {
        return calculate_global_blurriness(&lap, gray_img, texture);
    }

    let mut block_variances = Vec::with_capacity((grid_size * grid_size) as usize);

    for by in 0..grid_size {
        for bx in 0..grid_size {
            let x_start = bx * block_w;
            let y_start = by * block_h;

            let mut sum = 0.0;
            let mut sum_sq = 0.0;
            let mut count = 0.0;

            for y in y_start..(y_start + block_h) {
                for x in x_start..(x_start + block_w) {
                    let val = f64::from(lap.get_pixel(x, y)[0]);
                    sum += val;
                    sum_sq = val.mul_add(val, sum_sq);
                    count += 1.0;
                }
            }

            if count > 1.0 {
                let mean = sum / count;
                let variance = mean.mul_add(-mean, sum_sq / count);
                block_variances.push(variance);
            }
        }
    }

    // Sort descending to find the sharpest regions
    block_variances.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    // Take the top 10% sharpest blocks to represent overall focus level
    let top_k = std::cmp::max(1, block_variances.len() / 10);
    let top_sum: f64 = block_variances.iter().take(top_k).sum();
    let representative_variance = top_sum / (top_k as f64);

    let contrast = calculate_contrast(gray_img);
    let adjusted_var = representative_variance * contrast * 0.5f64.mul_add(-texture, 1.0);

    let raw_score = ((adjusted_var - 50.0) / 1200.0).clamp(0.0, 1.0);

    // Apply a square root to soften the penalty for slightly out-of-focus images.
    raw_score.sqrt()
}

fn calculate_global_blurriness(
    lap: &ImageBuffer<Luma<i16>, Vec<i16>>,
    gray_img: &GrayImage,
    texture: f64,
) -> f64 {
    let n = f64::from(lap.width() * lap.height());
    if n < 2.0 {
        return 0.0;
    }

    let mean = lap.pixels().map(|p| f64::from(p[0])).sum::<f64>() / n;
    let variance = lap
        .pixels()
        .map(|p| {
            let v = f64::from(p[0]) - mean;
            v * v
        })
        .sum::<f64>()
        / (n - 1.0);

    let adjusted_var = variance * calculate_contrast(gray_img) * 0.5f64.mul_add(-texture, 1.0);

    let raw_score = ((adjusted_var - 50.0) / 950.0).clamp(0.0, 1.0);

    raw_score.powf(0.4)
}

fn calculate_noise(gray_img: &GrayImage, texture: f64) -> f64 {
    let denoised = median_filter(gray_img, 5, 5);
    let n = f64::from(gray_img.width() * gray_img.height());

    let mean_diff = gray_img
        .pixels()
        .zip(denoised.pixels())
        .map(|(p, d)| (f64::from(p[0]) - f64::from(d[0])).abs())
        .sum::<f64>()
        / n;

    let raw_noise = (1.0 - ((mean_diff - 2.0) / 13.0)).clamp(0.0, 1.0);
    raw_noise * 0.5f64.mul_add(-texture, 1.0)
}

/// Measures exposure health using dynamic range spread (middle 90% of the histogram).
fn calculate_exposure(gray_img: &GrayImage) -> f64 {
    let mut hist = [0u64; 256];
    for p in gray_img.pixels() {
        hist[p[0] as usize] += 1;
    }

    let total_pixels = u64::from(gray_img.width()) * u64::from(gray_img.height());
    if total_pixels == 0 {
        return 0.0;
    }

    let mut running_sum = 0u64;
    let mut p5 = 0;
    let mut p95 = 255;
    let mut lower_bound_reached = false;

    let target_lower = total_pixels * 5 / 100;
    let target_upper = total_pixels * 95 / 100;

    for (i, &count) in hist.iter().enumerate() {
        running_sum += count;
        if !lower_bound_reached && running_sum >= target_lower {
            p5 = i;
            lower_bound_reached = true;
        }
        if running_sum >= target_upper {
            p95 = i;
            break;
        }
    }

    let spread = (p95 as f64) - (p5 as f64);
    let spread_score = (spread / 110.0).clamp(0.0, 1.0);

    let overexposure_penalty = if p5 > 180 {
        let factor = ((p5 - 180) as f64) / 60.0;
        (1.0 - factor).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let underexposure_penalty = if p95 < 45 {
        let factor = ((45 - p95) as f64) / 30.0;
        (1.0 - factor).clamp(0.0, 1.0)
    } else {
        1.0
    };

    spread_score * overexposure_penalty * underexposure_penalty
}

/// Calculates a soft accidentalness score between 0.0 (likely intentional) and 1.0 (likely pocket or blank flash).
fn calculate_accidentalness(gray_img: &GrayImage) -> f64 {
    let n = f64::from(gray_img.width() * gray_img.height());
    if n == 0.0 {
        return 0.0;
    }

    let sum = gray_img.pixels().map(|p| f64::from(p[0])).sum::<f64>();
    let mean = sum / n;

    let variance = gray_img
        .pixels()
        .map(|p| {
            let diff = f64::from(p[0]) - mean;
            diff * diff
        })
        .sum::<f64>()
        / n;
    let std_dev = variance.sqrt();

    // Case 1: Dark and highly uniform (pocket shot, inside bag, lens cap)
    // Mean transition range: 0.0 to 25.0
    // Standard deviation transition range: 0.0 to 10.0
    let dark_factor = (1.0 - (mean / 25.0)).clamp(0.0, 1.0);
    let dark_uniform_factor = (1.0 - (std_dev / 10.0)).clamp(0.0, 1.0);
    let pocket_score = dark_factor * dark_uniform_factor;

    // Case 2: Bright and highly uniform (accidental flash close-up, blank overexposed screen)
    // Mean transition range: 230.0 to 255.0
    // Standard deviation transition range: 0.0 to 10.0
    let bright_factor = ((mean - 230.0) / 25.0).clamp(0.0, 1.0);
    let bright_uniform_factor = (1.0 - (std_dev / 10.0)).clamp(0.0, 1.0);
    let flashlight_score = bright_factor * bright_uniform_factor;

    pocket_score.max(flashlight_score)
}
