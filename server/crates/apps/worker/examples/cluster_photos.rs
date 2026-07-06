use app_state::{database_url, load_app_settings};
use color_eyre::Result;
use color_eyre::eyre::{Context, eyre};
use common_services::database::get_db_pool;
use pgvector::Vector;
use sqlx::FromRow;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};
use worker::handlers::common::clustering::run_hdbscan;

/// A simple struct to hold the necessary data for clustering and file operations.
#[derive(Debug, FromRow)]
struct PhotoData {
    relative_path: String,
    embedding: Vector,
}

#[tokio::main]
async fn main() -> Result<()> {
    // --- Configuration ---
    const USER_ID_TO_TEST: i32 = 1;
    const OUTPUT_DIR: &str = "photo_cluster_output";
    let settings = load_app_settings()?;

    // --- Setup ---
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let pool = get_db_pool(database_url(), true).await?;
    let output_path = PathBuf::from(OUTPUT_DIR);

    // --- 1. Fetch User and Photo Data ---
    info!("Fetching data for user_id = {}", USER_ID_TO_TEST);

    // Fetch all photo embeddings for the user.
    // We use DISTINCT ON to get only one embedding per media item, which is crucial for videos.
    let photos = sqlx::query_as::<_, PhotoData>(
        r"SELECT DISTINCT ON (media_item.id)
               media_item.relative_path,
               va.embedding
           FROM visual_analysis va
           JOIN media_item ON media_item.id = va.media_item_id
           WHERE media_item.user_id = $1 AND media_item.deleted = false
           ORDER BY media_item.id, va.created_at",
    )
    .bind(USER_ID_TO_TEST)
    .fetch_all(&pool)
    .await?;

    if photos.is_empty() {
        info!("No photos found for user {}. Exiting.", USER_ID_TO_TEST);
        return Ok(());
    }
    info!("Found {} photos to process.", photos.len());

    // --- 2. Run Clustering Algorithm ---
    info!("Running HDBSCAN clustering...");
    let embeddings: Vec<Vec<f32>> = photos.iter().map(|p| p.embedding.to_vec()).collect();
    let (labels, _centroids) = run_hdbscan(&embeddings, 2, 2)?;
    info!(
        "Clustering complete. Found {} clusters (excluding noise).",
        labels
            .iter()
            .filter(|&&l| l >= 0)
            .max()
            .map_or(0, |max| max + 1)
    );

    // --- 3. Group Photos by Cluster ID ---
    let mut clusters: HashMap<i32, Vec<&PhotoData>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        // label -1 represents noise points, which we'll place in a separate folder.
        clusters.entry(label).or_default().push(&photos[i]);
    }

    // --- 4. Organize Files into Directories ---
    info!(
        "Organizing photos into output directory: {:?}",
        &output_path
    );
    prepare_output_directory(&output_path)?;

    for (cluster_id, photos_in_cluster) in clusters {
        let cluster_dir_name = if cluster_id == -1 {
            "noise".to_string()
        } else {
            format!("cluster_{cluster_id:03}") // Padded for better sorting
        };

        let cluster_path = output_path.join(&cluster_dir_name);
        fs::create_dir_all(&cluster_path)?;

        info!(
            "Copying {} photos into '{}'",
            photos_in_cluster.len(),
            cluster_dir_name
        );

        for photo in photos_in_cluster {
            let source_path = &settings.ingest.media_root.join(&photo.relative_path);

            let file_name = Path::new(&photo.relative_path)
                .file_name()
                .ok_or_else(|| eyre!("Could not get file name from {}", photo.relative_path))?;

            let dest_path = cluster_path.join(file_name);

            // Copy the file. This is cross-platform but uses more disk space than symlinks.
            if let Err(e) = fs::copy(source_path, &dest_path) {
                error!(
                    "Warning: Could not copy file from {:?} to {:?}: {}. Skipping.",
                    source_path, dest_path, e
                );
            }
        }
    }

    info!(
        "Successfully organized photos into cluster directories at: {}",
        output_path.display()
    );
    Ok(())
}

/// Cleans and creates the main output directory for a fresh run.
fn prepare_output_directory(path: &Path) -> Result<()> {
    if path.exists() {
        info!("Output directory exists. Removing it to ensure a clean run.");
        fs::remove_dir_all(path).context("Failed to remove existing output directory")?;
    }
    fs::create_dir_all(path).context("Failed to create output directory")?;
    Ok(())
}
