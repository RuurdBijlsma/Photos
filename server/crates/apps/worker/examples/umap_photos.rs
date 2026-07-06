#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use app_state::database_url;
use color_eyre::eyre::Result;
use common_services::database::get_db_pool;
use ndarray::Array2;
use plotters::prelude::*;
use rand::RngExt;
use rayon::prelude::*;
use sqlx::{FromRow, PgPool};
use std::fs;
use std::path::Path;
use tracing::info;
use umap_rs::{GraphParams, Umap, UmapConfig};

#[derive(FromRow, Debug)]
struct PhotoEmbeddingRow {
    embedding: pgvector::Vector,
}

// Fetch embeddings without join logic
async fn fetch_photo_embeddings(pool: &PgPool, user_id: i32) -> Result<Vec<PhotoEmbeddingRow>> {
    let rows = sqlx::query_as::<_, PhotoEmbeddingRow>(
        r"
        SELECT
            id,
            media_item_id,
            embedding
        FROM visual_analysis
        WHERE user_id = $1 AND deleted = false
        ",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// Computes the exact K-Nearest Neighbors in parallel on the CPU
fn compute_exact_knn(data: &Array2<f32>, k: usize) -> (Array2<u32>, Array2<f32>) {
    let n_samples = data.nrows();
    let mut knn_indices = Array2::zeros((n_samples, k));
    let mut knn_dists = Array2::zeros((n_samples, k));

    let rows: Vec<_> = data.rows().into_iter().collect();

    // Parallel search using rayon across CPU cores
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

            // Sort ascending by distance
            dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            // Keep top k neighbors (including self-distance at 0.0)
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

// Generate random points for the optimizer's starting embedding positions
fn random_init(n_samples: usize, n_components: usize) -> Array2<f32> {
    let mut rng = rand::rng();
    Array2::from_shape_fn((n_samples, n_components), |_| {
        rng.random_range(-10.0..10.0f32)
    })
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let pool = get_db_pool(database_url(), true).await?;

    // --- Configuration ---
    let user_id = 1;
    let n_neighbors = 15; // Default KNN size
    let output_dir = Path::new("umap_output");
    let output_file = output_dir.join("photo_projection_cpu.png");

    info!(
        "Fetching photo embeddings from database for user {}...",
        user_id
    );
    let photo_data = fetch_photo_embeddings(&pool, user_id).await?;
    if photo_data.is_empty() {
        info!(
            "No photo embeddings found for user_id = {}. Ensure you have run analysis.",
            user_id
        );
        return Ok(());
    }
    let n_samples = photo_data.len();
    info!("Fetched {} photo embeddings.", n_samples);

    // Build the ndarray Array2 input
    let n_features = 768; // Dimension of the visual analysis embeddings
    let mut data_vec = Vec::with_capacity(n_samples * n_features);
    for row in &photo_data {
        data_vec.extend(row.embedding.to_vec());
    }
    let data = Array2::from_shape_vec((n_samples, n_features), data_vec)?;

    info!("Computing exact KNN graph parallelized on CPU...");
    let (knn_indices, knn_dists) = compute_exact_knn(&data, n_neighbors);

    info!("Initializing initial embedding coordinates...");
    let init = random_init(n_samples, 2);

    info!("Fitting UMAP using CPU (umap-rs engine)...");
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
    info!("UMAP reduction complete. Generating visualization...");
    fs::create_dir_all(output_dir)?;

    // Extract raw 2D coordinate points
    let mut points = Vec::with_capacity(n_samples);
    for idx in 0..n_samples {
        let x = f64::from(embedding_2d[[idx, 0]]);
        let y = f64::from(embedding_2d[[idx, 1]]);
        points.push((x, y));
    }

    // Set up plotting canvas boundaries
    let root = BitMapBackend::new(&output_file, (1200, 900)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for &(x, y) in &points {
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
    }

    // Apply 5% padding around coordinates
    let pad_x = (max_x - min_x) * 0.05;
    let pad_y = (max_y - min_y) * 0.05;
    let min_x = min_x - pad_x;
    let max_x = max_x + pad_x;
    let min_y = min_y - pad_y;
    let max_y = max_y + pad_y;

    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Photo Embeddings 2D UMAP Projection (umap-rs)",
            ("sans-serif", 32).into_font(),
        )
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart.configure_mesh().draw()?;

    // Draw all points with a single uniform color style
    let point_style = RGBColor(31, 119, 180); // Classic blue tone
    chart.draw_series(
        points
            .iter()
            .map(|&(x, y)| Circle::new((x, y), 5, ShapeStyle::from(&point_style).filled())),
    )?;

    root.present()?;
    info!(
        "Successfully generated 2D UMAP projection and saved it to: {:?}",
        output_file
    );

    Ok(())
}
