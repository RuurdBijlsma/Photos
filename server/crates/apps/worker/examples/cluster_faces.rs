#![allow(
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use color_eyre::eyre::Result;

use ab_glyph::FontArc;
use app_state::{database_url, load_app_settings, to_posix_string};
use common_services::database::get_db_pool;
use common_services::database::visual_analysis::face::FaceEmbedding;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use sqlx::{FromRow, PgPool};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;
use worker::handlers::common::clustering::{group_by_cluster, run_hdbscan};

// A new struct to hold more comprehensive face details for drawing
#[derive(FromRow, Debug)]
struct FaceDetails {
    id: i64,
    media_item_id: String,
    position_x: f32,
    position_y: f32,
    width: f32,
    height: f32,
    embedding: pgvector::Vector,
    relative_path: String,
}

// Function to fetch all face details for a user, including the media item path
async fn fetch_all_face_details(pool: &PgPool, user_id: i32) -> Result<Vec<FaceDetails>> {
    // This query assumes you have a `media_item` table with a `path` column.
    // Adjust the query if your schema is different.
    let faces = sqlx::query_as::<_, FaceDetails>(
        r"
        SELECT
            f.id,
            va.media_item_id,
            f.position_x,
            f.position_y,
            f.width,
            f.height,
            f.embedding,
            mi.relative_path
        FROM face f
        JOIN visual_analysis va ON f.visual_analysis_id = va.id
        JOIN media_item mi ON va.media_item_id = mi.id
        WHERE mi.user_id = $1
        ",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(faces)
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    let settings = load_app_settings()?;
    let pool = get_db_pool(database_url(), true).await?;

    // --- Configuration ---
    let user_id_to_debug = 1;
    let output_directory = Path::new("face_clusters_output");

    // --- 1. Fetch Data ---
    info!("Fetching data for user_id: {}", user_id_to_debug);
    let all_faces = fetch_all_face_details(&pool, user_id_to_debug).await?;

    if all_faces.is_empty() {
        info!("No faces found for this user.");
        return Ok(());
    }

    // Create the FaceEmbedding list needed for the clustering functions
    let face_embeddings_for_clustering: Vec<FaceEmbedding> = all_faces
        .iter()
        .map(|f| FaceEmbedding {
            id: f.id,
            media_item_id: f.media_item_id.clone(),
            embedding: f.embedding.clone(),
        })
        .collect();

    let embeddings: Vec<Vec<f32>> = face_embeddings_for_clustering
        .iter()
        .map(|f| f.embedding.to_vec())
        .collect();

    // --- 2. Run Clustering ---
    info!("Running HDBSCAN clustering...");
    let (labels, _new_centroids) = run_hdbscan(&embeddings, 4, 5)?;
    info!("Clustering complete.");

    let clusters = group_by_cluster(&labels, &face_embeddings_for_clustering);

    // --- 3. Prepare for Drawing ---
    if output_directory.exists() {
        fs::remove_dir_all(output_directory)?;
    }
    fs::create_dir_all(output_directory)?;
    info!("Output will be saved to: {:?}", output_directory);

    // Load a font for drawing labels using ab_glyph
    // Update this path if it's incorrect for your system.
    let font_data = include_bytes!("assets/DejaVuSans.ttf");
    let font = FontArc::try_from_slice(font_data)?;

    // This map will store images that need to be modified. We process them at the end.
    let mut images_to_draw: HashMap<String, (RgbImage, Vec<(Rect, String)>)> = HashMap::new();

    // --- 4. Process Faces and Prepare for Drawing ---
    for (cluster_id, faces_in_cluster) in clusters {
        let cluster_label = format!("Cluster {cluster_id}");
        info!(
            "Processing {} with {} faces",
            cluster_label,
            faces_in_cluster.len()
        );

        for face_embedding in faces_in_cluster {
            let face_details = all_faces
                .iter()
                .find(|f| f.id == face_embedding.id)
                .unwrap();
            let original_image_path =
                to_posix_string(&settings.ingest.media_root.join(&face_details.relative_path));

            // Load the image if it's not already in our map
            let (image, _annotations) = images_to_draw
                .entry(original_image_path.clone())
                .or_insert_with(|| {
                    let img = image::open(original_image_path.clone())
                        .expect("Failed to open image")
                        .to_rgb8();
                    (img, Vec::new())
                });

            let (img_width, img_height) = image.dimensions();

            // Convert normalized coordinates to pixel coordinates
            let x = (face_details.position_x * img_width as f32) as i32;
            let y = (face_details.position_y * img_height as f32) as i32;
            let width = (face_details.width * img_width as f32) as u32;
            let height = (face_details.height * img_height as f32) as u32;

            let rect = Rect::at(x, y).of_size(width, height);

            // Add the rectangle and label to the list for this image
            images_to_draw
                .get_mut(&original_image_path)
                .unwrap()
                .1
                .push((rect, cluster_label.clone()));
        }
    }

    // --- 5. Draw Annotations and Save Images ---
    for (path_str, (mut image, annotations)) in images_to_draw {
        info!("Drawing {} annotations on {}", annotations.len(), path_str);
        for (rect, label) in annotations {
            let color = Rgb([255u8, 0, 0]); // Red color for the box
            draw_hollow_rect_mut(&mut image, rect, color);

            // The scale is now a simple f32 value for the font size in pixels.
            let font_scale = 70.0;
            let text_y_position = if rect.top() < (font_scale as i32 + 5) {
                rect.bottom()
            } else {
                rect.top() - (font_scale as i32 + 5)
            };

            // The call to draw_text_mut is now updated
            draw_text_mut(
                &mut image,
                Rgb([0u8, 255u8, 0u8]),
                rect.left(),
                text_y_position,
                font_scale,
                &font,
                &label,
            );
        }

        let file_name = Path::new(&path_str).file_name().unwrap();

        // Create a subdirectory for each cluster to avoid overwriting images
        // For simplicity, we'll save all annotated images to the root output folder.
        // If an image has faces from multiple clusters, it will be annotated for all of them.
        let output_path = output_directory.join(file_name);
        image.save(&output_path)?;
        info!("Saved annotated image to {:?}", output_path);
    }

    info!("\nDebug script finished!");
    Ok(())
}
