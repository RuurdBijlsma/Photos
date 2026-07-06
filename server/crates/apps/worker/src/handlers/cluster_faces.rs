use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::clustering::{self, ClusterEntity};
use crate::handlers::common::utils::get_images_to_analyze;
use color_eyre::{Result, eyre::eyre};
use common_services::api::album::service::get_representative_thumbnail;
use common_services::database::jobs::Job;
use common_services::database::media_item_store::MediaItemStore;
use common_services::database::person::ExistingFaceCluster;
use common_services::utils::nice_id;
use face_id::detector::BoundingBox;
use face_id::helpers::extract_face_thumbnail;
use generate_thumbnails::ffmpeg::FfmpegCommand;
use itertools::Itertools;
use pgvector::Vector;
use sqlx::{PgPool, Transaction, query, query_as};
use std::collections::{HashMap, HashSet};
use tempfile::Builder;
use tracing::info;

const ENTITY_NAME: &str = "face";
const MIN_ITEMS_TO_CLUSTER: usize = 4;
const MIN_SAMPLES: usize = 5;
const CENTROID_MATCH_THRESHOLD: f32 = 0.6;
const THUMBNAIL_SIZE: u32 = 256;
const PADDING_FACTOR: f32 = 1.8;

impl ClusterEntity for ExistingFaceCluster {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn centroid(&self) -> Option<&Vector> {
        self.centroid.as_ref()
    }
}

#[derive(Clone)]
struct FaceToCluster {
    id: i64,
    media_item_id: String,
    embedding: Vector,
    position_x: f32,
    position_y: f32,
    width: f32,
    height: f32,
    percentage: i32,
}

async fn fetch_existing_clusters(pool: &PgPool, user_id: i32) -> Result<Vec<ExistingFaceCluster>> {
    // Join through person to ensure we only get clusters for this user
    query_as!(
        ExistingFaceCluster,
        r#"SELECT fc.id, fc.person_id, fc.centroid as "centroid: _"
           FROM face_cluster fc
           JOIN person p ON fc.person_id = p.id
           WHERE p.user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

async fn fetch_embeddings(pool: &PgPool, user_id: i32) -> Result<Vec<FaceToCluster>> {
    query_as!(
        FaceToCluster,
        r#"SELECT
               f.id,
               va.media_item_id,
               f.embedding as "embedding!: Vector",
               f.position_x,
               f.position_y,
               f.width,
               f.height,
               va.percentage
           FROM face f
           JOIN visual_analysis va ON f.visual_analysis_id = va.id
           JOIN media_item mi ON mi.id = va.media_item_id
           WHERE mi.user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

async fn extract_and_save_cluster_thumbnail(
    context: &WorkerContext,
    face: &FaceToCluster,
    cluster_id: &str,
) -> Result<()> {
    let relative_path =
        MediaItemStore::find_relative_path_by_id(&context.pool, &face.media_item_id)
            .await
            .map_err(|e| eyre!("Database error: {}", e))?
            .ok_or_else(|| eyre!("Media item not found"))?;

    let media_root = &context.settings.ingest.media_root;
    let file_path = media_root.join(&relative_path);

    let images = get_images_to_analyze(
        context,
        &file_path,
        &face.media_item_id,
        &[face.percentage as u64],
        context
            .settings
            .ingest
            .thumbnails
            .heights
            .iter()
            .max()
            .copied(),
    );

    let (_, image_path) = images.into_iter().next().ok_or_else(|| {
        eyre!(
            "No image found for analysis at percentage {}",
            face.percentage
        )
    })?;

    let temp_file = Builder::new().suffix(".png").tempfile()?;
    let temp_path = temp_file.path().to_path_buf();

    let mut ffmpeg = FfmpegCommand::new(&image_path);
    ffmpeg.map_still_output("0:v", &temp_path);
    ffmpeg.run().await?;

    let img = image::ImageReader::open(&temp_path)?
        .with_guessed_format()?
        .decode()?;

    let bbox = BoundingBox {
        x1: face.position_x,
        y1: face.position_y,
        x2: face.position_x + face.width,
        y2: face.position_y + face.height,
    };

    let thumb = extract_face_thumbnail(&img, &bbox, PADDING_FACTOR, THUMBNAIL_SIZE);

    let out_dir = &context.settings.ingest.face_clusters_root;
    tokio::fs::create_dir_all(out_dir).await?;

    let out_path = out_dir.join(format!("{cluster_id}.webp"));
    thumb.save(out_path)?;

    Ok(())
}

async fn get_representative_face(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    faces_in_cluster: &[&FaceToCluster],
) -> Result<FaceToCluster> {
    let media_items_in_cluster = faces_in_cluster
        .iter()
        .map(|f| f.media_item_id.clone())
        .unique()
        .collect::<Vec<_>>();
    if let Some(representative_media_item_id) =
        get_representative_thumbnail(tx, &media_items_in_cluster).await?
        && let Some(face) = faces_in_cluster
            .iter()
            .find(|f| f.media_item_id == representative_media_item_id)
    {
        return Ok((*face).clone());
    }
    get_biggest_face(faces_in_cluster)
}

fn get_biggest_face(faces_in_cluster: &[&FaceToCluster]) -> Result<FaceToCluster> {
    faces_in_cluster
        .iter()
        .max_by(|a, b| {
            let area_a = a.width * a.height;
            let area_b = b.width * b.height;
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|f| (*f).clone())
        .ok_or_else(|| eyre!("No faces in cluster"))
}

async fn upsert_and_link(
    context: &WorkerContext,
    tx: &mut Transaction<'_, sqlx::Postgres>,
    user_id: i32,
    clusters: HashMap<usize, Vec<&FaceToCluster>>,
    new_centroids: &[Vec<f32>],
    cluster_map: &HashMap<usize, String>, // Map of cluster_idx -> face_cluster.id (String)
) -> Result<()> {
    for (cluster_idx, faces_in_cluster) in clusters {
        let face_ids: Vec<i64> = faces_in_cluster.iter().map(|f| f.id).collect();
        let new_centroid = new_centroids
            .get(cluster_idx)
            .map(|v| Vector::from(v.clone()));
        let representative_face = get_representative_face(tx, &faces_in_cluster).await?;

        let cluster_id = if let Some(existing_cluster_id) = cluster_map.get(&cluster_idx) {
            // Update existing cluster
            query("UPDATE face_cluster SET centroid = $1, updated_at = now() WHERE id = $2")
                .bind(new_centroid)
                .bind(existing_cluster_id)
                .execute(&mut **tx)
                .await?;
            existing_cluster_id.clone()
        } else {
            // New discovery: Create Person AND Cluster
            let new_person_id = nice_id(10);
            let new_cluster_id = nice_id(10);

            query!(
                "INSERT INTO person (id, user_id) VALUES ($1, $2)",
                new_person_id,
                user_id
            )
            .execute(&mut **tx)
            .await?;

            query("INSERT INTO face_cluster (id, user_id, person_id, centroid, thumb_media_item_id) VALUES ($1, $2, $3, $4, $5)")
                .bind(&new_cluster_id)
                .bind(user_id)
                .bind(&new_person_id)
                .bind(new_centroid)
                .bind(&representative_face.media_item_id)
                .execute(&mut **tx)
                .await?;

            // Now update the person to point to this cluster for its thumbnail
            query!(
                "UPDATE person SET face_thumb_id = $1 WHERE id = $2",
                new_cluster_id,
                new_person_id
            )
            .execute(&mut **tx)
            .await?;

            new_cluster_id
        };

        // Link faces to the cluster
        query!(
            "UPDATE face SET face_cluster_id = $1 WHERE id = ANY($2)",
            cluster_id,
            &face_ids
        )
        .execute(&mut **tx)
        .await?;

        extract_and_save_cluster_thumbnail(context, &representative_face, &cluster_id).await?;
    }
    Ok(())
}

async fn cleanup_obsolete(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    existing_clusters: &[ExistingFaceCluster],
    matched_ids: &HashSet<String>,
) -> Result<()> {
    let obsolete_cluster_ids: Vec<String> = existing_clusters
        .iter()
        .filter(|c| !matched_ids.contains(&c.id))
        .map(|c| c.id.clone())
        .collect();

    if !obsolete_cluster_ids.is_empty() {
        // Unlink faces from these clusters
        query!(
            "UPDATE face SET face_cluster_id = NULL WHERE face_cluster_id = ANY($1)",
            &obsolete_cluster_ids
        )
        .execute(&mut **tx)
        .await?;

        query!(
            "DELETE FROM face_cluster WHERE id = ANY($1)",
            &obsolete_cluster_ids
        )
        .execute(&mut **tx)
        .await?;
    }

    query!(
        "DELETE FROM person p
             WHERE NOT EXISTS (SELECT 1 FROM face_cluster fc WHERE fc.person_id = p.id)"
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Checks if there have been any updates to media items or new visual analysis records
/// since the last successful face cluster generation for the user.
async fn needs_clustering(pool: &PgPool, user_id: i32) -> Result<bool> {
    let needs_run = sqlx::query_scalar!(
        r#"
        WITH last_run AS (
            SELECT MAX(updated_at) AS last_run_time
            FROM face_cluster
            WHERE user_id = $1
        )
        SELECT
            CASE
                -- If there is no record of previous runs, clustering must run
                WHEN (SELECT last_run_time FROM last_run) IS NULL THEN TRUE
                ELSE
                    -- Check if any media items were created or updated since the last run
                    -- Soft deletions are also detected, actual row deletions are not
                    EXISTS (
                        SELECT 1 FROM media_item
                        WHERE user_id = $1 AND updated_at > (SELECT last_run_time FROM last_run)
                    )
            END AS "needs_run!"
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(needs_run)
}

pub async fn handle(context: &WorkerContext, job: &Job) -> Result<JobResult> {
    let user_ids = clustering::fetch_user_ids(&context.pool, job).await?;

    for user_id in user_ids {
        // Skip user if there are no new updates or changes since the last run
        if !needs_clustering(&context.pool, user_id).await? {
            info!(
                "Skipping face clustering for user {} - no updates detected",
                user_id
            );
            continue;
        }

        let existing_clusters = fetch_existing_clusters(&context.pool, user_id).await?;
        let items_to_cluster = fetch_embeddings(&context.pool, user_id).await?;
        if items_to_cluster.len() < MIN_ITEMS_TO_CLUSTER {
            continue;
        }
        let embeddings: Vec<Vec<f32>> = items_to_cluster
            .iter()
            .map(|f| f.embedding.to_vec())
            .collect();

        // Run clustering
        let (labels, new_centroids) =
            clustering::run_hdbscan(&embeddings, MIN_ITEMS_TO_CLUSTER, MIN_SAMPLES)?;
        let cluster_map = clustering::match_centroids(
            &new_centroids,
            &existing_clusters,
            CENTROID_MATCH_THRESHOLD,
        )?;
        let new_clusters = clustering::group_by_cluster(&labels, &items_to_cluster);

        // Perform DB updates
        let mut tx = context.pool.begin().await?;
        upsert_and_link(
            context,
            &mut tx,
            user_id,
            new_clusters,
            &new_centroids,
            &cluster_map,
        )
        .await?;

        // Cleanup
        let matched_old_ids: HashSet<String> = cluster_map.values().cloned().collect();
        cleanup_obsolete(&mut tx, &existing_clusters, &matched_old_ids).await?;

        tx.commit().await?;
        info!("Reconciled {} clusters for user {}", ENTITY_NAME, user_id);
    }

    Ok(JobResult::Done)
}
