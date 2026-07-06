#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use app_state::{database_url, load_app_settings};
use color_eyre::eyre::Result;
use common_services::database::get_db_pool;
use ndarray::Array2;
use rand::RngExt;
use rayon::prelude::*;
use sqlx::{FromRow, PgPool};
use std::fs;
use std::path::Path;
use tracing::{error, info};
use umap_rs::{GraphParams, Umap, UmapConfig};

// ==========================================
// --- Configuration Constants ---
// ==========================================
const USER_ID: i32 = 1;

/// The number of random photos (embeddings) to sample from the database.
const N_SAMPLES: usize = 2000;

/// Output image width in pixels.
const CANVAS_WIDTH: u32 = 4000;

/// Output image height in pixels.
const CANVAS_HEIGHT: u32 = 3000;

/// The bounding size (max dimension) of each thumbnail pasted onto the plot.
const THUMB_SIZE: u32 = 120;
// ==========================================

#[derive(FromRow, Debug)]
struct SampledPhotoRow {
    relative_path: String,
    embedding: pgvector::Vector,
}

/// Fetch N random photo embeddings along with their relative paths.
async fn fetch_random_photo_embeddings(
    pool: &PgPool,
    user_id: i32,
    limit: usize,
) -> Result<Vec<SampledPhotoRow>> {
    // We use a CTE to ensure we isolate unique photos before sampling them randomly
    let rows = sqlx::query_as::<_, SampledPhotoRow>(
        r"
        WITH unique_items AS (
            SELECT DISTINCT ON (media_item.id)
                media_item.relative_path,
                va.embedding
            FROM visual_analysis va
            JOIN media_item ON media_item.id = va.media_item_id
            WHERE media_item.user_id = $1 AND media_item.deleted = false AND media_item.is_video = false
            ORDER BY media_item.id, va.created_at
        )
        SELECT relative_path, embedding
        FROM unique_items
        ORDER BY RANDOM()
        LIMIT $2
        ",
    )
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?;

    Ok(rows)
}

/// Computes the exact K-Nearest Neighbors in parallel on the CPU
fn compute_exact_knn(data: &Array2<f32>, k: usize) -> (Array2<u32>, Array2<f32>) {
    let n_samples = data.nrows();
    let mut knn_indices = Array2::zeros((n_samples, k));
    let mut knn_dists = Array2::zeros((n_samples, k));

    let rows: Vec<_> = data.rows().into_iter().collect();

    let results: Vec<(Vec<u32>, Vec<f32>)> = rows
        .par_iter()
        .map(|row_i| {
            let mut dists: Vec<(usize, f32)> = rows
                .iter()
                .enumerate()
                .map(|(j, row_j)| {
                    let diff = row_i - row_j;
                    let dist_sq = diff.fold(0.0, |acc, &x| x.mul_add(x, acc));
                    (j, dist_sq.sqrt())
                })
                .collect();

            dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let k_neighbors = dists.into_iter().take(k).collect::<Vec<_>>();

            let mut idxs = Vec::with_capacity(k);
            let mut distances = Vec::with_capacity(k);
            for (idx, dist) in k_neighbors {
                idxs.push(idx as u32);
                distances.push(dist);
            }
            (idxs, distances)
        })
        .collect();

    for (i, (idxs, dsts)) in results.into_iter().enumerate() {
        for j in 0..k {
            knn_indices[[i, j]] = idxs[j];
            knn_dists[[i, j]] = dsts[j];
        }
    }

    (knn_indices, knn_dists)
}

/// Generate random starting positions for the UMAP optimizer
fn random_init(n_samples: usize, n_components: usize) -> Array2<f32> {
    let mut rng = rand::rng();
    Array2::from_shape_fn((n_samples, n_components), |_| {
        rng.random_range(-10.0..10.0f32)
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let settings = load_app_settings()?;
    let pool = get_db_pool(database_url(), true).await?;

    info!(
        "Sampling up to {} random photo embeddings for user {}...",
        N_SAMPLES, USER_ID
    );
    let photo_data = fetch_random_photo_embeddings(&pool, USER_ID, N_SAMPLES).await?;

    let n_samples = photo_data.len();
    if n_samples < 2 {
        info!(
            "Not enough photos found (found {}). Need at least 2 to run dimensionality reduction.",
            n_samples
        );
        return Ok(());
    }
    info!("Successfully fetched {} photos.", n_samples);

    // --- Build input dataset matrix ---
    let n_features = 768; // Dimension of the visual analysis embeddings
    let mut data_vec = Vec::with_capacity(n_samples * n_features);
    for row in &photo_data {
        data_vec.extend(row.embedding.to_vec());
    }
    let data = Array2::from_shape_vec((n_samples, n_features), data_vec)?;

    // Dynamically adjust neighbors depending on sample count (KNN k must be less than sample count)
    let n_neighbors = std::cmp::min(15, n_samples - 1);
    info!(
        "Computing exact KNN graph parallelized on CPU (k = {})...",
        n_neighbors
    );
    let (knn_indices, knn_dists) = compute_exact_knn(&data, n_neighbors);

    info!("Initializing coordinate layout space...");
    let init = random_init(n_samples, 2);

    info!("Fitting UMAP (umap-rs engine)...");
    let config = UmapConfig {
        n_components: 2,
        graph: GraphParams {
            n_neighbors,
            ..Default::default()
        },
        ..Default::default()
    };

    let umap_model = Umap::new(config);
    let fitted_model = umap_model.fit(
        data.view(),
        knn_indices.view(),
        knn_dists.view(),
        init.view(),
    );

    let embedding_2d = fitted_model.embedding();
    info!("UMAP reduction complete. Loading and resizing images in parallel...");

    // --- Parallel processing: Load and resize images ---
    let thumbnails: Vec<(usize, image::RgbImage)> = photo_data
        .par_iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let full_path = settings.ingest.media_root.join(&row.relative_path);
            match image::open(&full_path) {
                Ok(img) => {
                    // Create high quality fast thumbnail keeping aspect ratio
                    let thumb = img.thumbnail(THUMB_SIZE, THUMB_SIZE).to_rgb8();
                    Some((idx, thumb))
                }
                Err(e) => {
                    error!("Skipping image {:?}: {}", full_path, e);
                    None
                }
            }
        })
        .collect();

    if thumbnails.is_empty() {
        info!("No images were successfully loaded from files. Exiting.");
        return Ok(());
    }

    // --- Collect coordinates for successfully loaded images ---
    let mut points = Vec::with_capacity(thumbnails.len());
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &(idx, _) in &thumbnails {
        let x = f64::from(embedding_2d[[idx, 0]]);
        let y = f64::from(embedding_2d[[idx, 1]]);

        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }

        points.push((idx, x, y));
    }

    // Handle divisions by zero if coordinates are identical
    let range_x = if (max_x - min_x).abs() < f64::EPSILON {
        1.0
    } else {
        max_x - min_x
    };
    let range_y = if (max_y - min_y).abs() < f64::EPSILON {
        1.0
    } else {
        max_y - min_y
    };

    info!("Generating visual 2D layout canvas...");
    // Initialize empty neutral gray background
    let mut canvas =
        image::RgbImage::from_pixel(CANVAS_WIDTH, CANVAS_HEIGHT, image::Rgb([240, 240, 240]));

    // Keep images within bounds by adding margin
    let margin = f64::from(THUMB_SIZE);

    for (idx, thumb) in thumbnails {
        if let Some(&(_, x, y)) = points.iter().find(|&&(p_idx, _, _)| p_idx == idx) {
            let norm_x = (x - min_x) / range_x;
            let norm_y = (y - min_y) / range_y;

            // Project coordinates onto canvas pixel coordinates
            let pixel_x = margin + norm_x * 2.0f64.mul_add(-margin, f64::from(CANVAS_WIDTH));
            // Flip y so that higher UMAP values point higher up in standard graph form
            let pixel_y = f64::from(CANVAS_HEIGHT)
                - margin
                - norm_y * 2.0f64.mul_add(-margin, f64::from(CANVAS_HEIGHT));

            // Calculate top-left placement coordinates for centering
            let px = (pixel_x - (f64::from(thumb.width()) / 2.0)) as i64;
            let py = (pixel_y - (f64::from(thumb.height()) / 2.0)) as i64;

            // Paint the image on top of the canvas
            image::imageops::overlay(&mut canvas, &thumb, px, py);
        }
    }

    let output_dir = Path::new("umap_output");
    fs::create_dir_all(output_dir)?;
    let output_file = output_dir.join("photo_projection_visual.png");
    canvas.save(&output_file)?;

    info!(
        "Successfully generated visual UMAP plot! Output saved to: {:?}",
        output_file
    );

    Ok(())
}
