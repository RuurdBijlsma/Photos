use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::cache::tag_vocab_cache::{load_tag_vocab_cache, save_tag_vocab_cache};
use crate::handlers::common::clustering::{self, ClusterEntity};
use app_state::constants::TAG_VOCAB_FOLDER;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use common_services::api::album::service::get_representative_thumbnail;
use common_services::database::jobs::Job;
use common_services::database::photo_cluster::ExistingPhotoCluster;
use common_services::database::visual_analysis::visual_analysis::MediaEmbedding;
use common_services::utils::nice_id;
use open_clip_inference::TextEmbedder;
use pgvector::Vector;
use sqlx::{PgPool, QueryBuilder, Transaction, query, query_as, query_scalar};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;
use tokio::fs;
use tracing::info;

const ENTITY_NAME: &str = "photo";
const MIN_ITEMS_TO_CLUSTER: usize = 4;
const MIN_SAMPLES: usize = 4;
const CENTROID_MATCH_THRESHOLD: f32 = 0.6;

impl ClusterEntity for ExistingPhotoCluster {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn centroid(&self) -> Option<&Vector> {
        self.centroid.as_ref()
    }
}

async fn load_vocab_labels() -> Result<Vec<String>> {
    let mut labels = Vec::new();
    let dir = Path::new(TAG_VOCAB_FOLDER);
    if dir.exists() && dir.is_dir() {
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).await?;
                for line in content.lines() {
                    labels.push(line.to_string());
                }
            }
        }
    }
    Ok(labels)
}

// Optimization: Simplified query to use an Index-Only Scan on 'object(tag)', avoiding the join
async fn load_object_tags(pool: &PgPool) -> Result<Vec<String>> {
    let tags = sqlx::query_scalar!(
        r#"SELECT DISTINCT o.tag
           FROM object o
           JOIN visual_analysis va ON va.id = o.visual_analysis_id
           WHERE  va.deleted = false"#
    )
    .fetch_all(pool)
    .await?;
    Ok(tags)
}

async fn load_all_tags(pool: &PgPool) -> Result<HashSet<String>> {
    let vocab_labels = load_vocab_labels().await?;
    let object_tags = load_object_tags(pool).await?;

    let mut deduplicated_tags = HashSet::new();

    // Combine, trim, and capitalize the first letter of each tag
    for tag in vocab_labels.into_iter().chain(object_tags) {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut chars = trimmed.chars();
        if let Some(first_char) = chars.next() {
            let capitalized = first_char.to_uppercase().to_string() + chars.as_str();
            deduplicated_tags.insert(capitalized);
        }
    }

    Ok(deduplicated_tags)
}

async fn load_tag_embeddings(
    pool: &PgPool,
    text_embedder: &TextEmbedder,
    model_id: &str,
) -> Result<()> {
    let tags = load_all_tags(pool).await?;

    let existing_tags: Vec<String> = sqlx::query_scalar!("SELECT tag FROM cluster_tags")
        .fetch_all(pool)
        .await?;

    let existing_tags_set: HashSet<String> = existing_tags.iter().cloned().collect();

    let tags_to_delete: Vec<String> = existing_tags
        .into_iter()
        .filter(|t| !tags.contains(t))
        .collect();

    let tags_to_process: Vec<String> = tags
        .iter()
        .filter(|t| !existing_tags_set.contains(*t))
        .cloned()
        .collect();

    if tags_to_delete.is_empty() && tags_to_process.is_empty() {
        return Ok(());
    }

    let mut new_embeddings = Vec::new();

    if !tags_to_process.is_empty() {
        let mut static_cache = load_tag_vocab_cache(model_id).await?;
        let mut cache_updated = false;
        let mut uncached_tags = Vec::new();

        for tag in tags_to_process {
            if let Some(embedding) = static_cache.get(&tag) {
                new_embeddings.push((tag, Vector::from(embedding.clone())));
            } else {
                uncached_tags.push(tag);
            }
        }

        if !uncached_tags.is_empty() {
            info!(
                "Embedding {} uncached tags with model {}",
                uncached_tags.len(),
                model_id
            );
            let batch_size = 64;
            for chunk in uncached_tags.chunks(batch_size) {
                let chunk_vec: Vec<String> = chunk.to_vec();
                let embeddings_array = text_embedder
                    .embed_texts(&chunk_vec)
                    .map_err(|e| eyre!("CLIP embedding generation failed: {:?}", e))?;

                for (i, tag) in chunk.iter().enumerate() {
                    let row: Vec<f32> = embeddings_array.row(i).iter().copied().collect();
                    static_cache.insert(tag.clone(), row.clone());
                    new_embeddings.push((tag.clone(), Vector::from(row)));
                }
            }
            cache_updated = true;
        }

        if cache_updated {
            save_tag_vocab_cache(model_id, &static_cache).await?;
        }
    }

    if !tags_to_delete.is_empty() || !new_embeddings.is_empty() {
        let mut tx = pool.begin().await?;

        if !tags_to_delete.is_empty() {
            sqlx::query!(
                "DELETE FROM cluster_tags WHERE tag = ANY($1::varchar[])",
                &tags_to_delete
            )
            .execute(&mut *tx)
            .await?;
        }

        if !new_embeddings.is_empty() {
            for chunk in new_embeddings.chunks(1000) {
                let mut query_builder: QueryBuilder<sqlx::Postgres> =
                    QueryBuilder::new("INSERT INTO cluster_tags (tag, embedding) ");

                query_builder.push_values(chunk, |mut b, (tag, embedding)| {
                    b.push_bind(tag.clone()).push_bind(embedding.clone());
                });

                query_builder
                    .push(" ON CONFLICT (tag) DO UPDATE SET embedding = EXCLUDED.embedding");

                let query = query_builder.build();
                query.execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;
    }

    Ok(())
}

async fn fetch_existing_clusters(pool: &PgPool, user_id: i32) -> Result<Vec<ExistingPhotoCluster>> {
    query_as!(
        ExistingPhotoCluster,
        r#"SELECT id, friendly_label, centroid as "centroid: _" FROM photo_cluster WHERE user_id = $1"#,
        user_id
    )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

async fn fetch_embeddings(pool: &PgPool, user_id: i32) -> Result<Vec<MediaEmbedding>> {
    query_as!(
        MediaEmbedding,
        r#"SELECT DISTINCT ON (media_item.id)
               media_item.id as media_item_id,
               va.embedding as "embedding!: Vector"
           FROM visual_analysis va
           JOIN media_item ON media_item.id = va.media_item_id
           WHERE media_item.user_id = $1 AND media_item.deleted = false
           ORDER BY media_item.id, va.created_at"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

async fn find_cluster_label(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    centroid: &[f32],
) -> Result<String> {
    let centroid_vector = Vector::from(centroid.to_owned());

    // Nearest neighbor search via cosine distance globally
    let label: Option<String> = sqlx::query_scalar!(
        "SELECT tag FROM cluster_tags
         ORDER BY embedding <=> $1
         LIMIT 1",
        centroid_vector as _
    )
    .fetch_optional(&mut **tx)
    .await?;

    Ok(label.unwrap_or_else(|| "Unknown".to_string()))
}

async fn upsert_and_link(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    user_id: i32,
    clusters: HashMap<usize, Vec<&MediaEmbedding>>,
    new_centroids: &[Vec<f32>],
    cluster_map: &HashMap<usize, String>,
) -> Result<()> {
    for (cluster_idx, photos_in_cluster) in clusters {
        let media_item_ids: Vec<String> = photos_in_cluster
            .iter()
            .map(|p| p.media_item_id.clone())
            .collect();
        let new_centroid_vec = new_centroids.get(cluster_idx);
        let new_centroid = new_centroid_vec.map(|v| Vector::from(v.clone()));
        let thumbnail_media_item_id = get_representative_thumbnail(
            tx,
            &photos_in_cluster
                .iter()
                .map(|f| f.media_item_id.clone())
                .collect::<Vec<String>>(),
        )
        .await?;

        let user_friendly_label = if let Some(centroid) = new_centroid_vec {
            Some(find_cluster_label(tx, centroid).await?)
        } else {
            None
        };

        let photo_cluster_id = if let Some(existing_id) = cluster_map.get(&cluster_idx) {
            query("UPDATE photo_cluster SET centroid = $1, thumbnail_media_item_id = $2, friendly_label = $3, updated_at = now() WHERE id = $4")
                .bind(&new_centroid).bind(thumbnail_media_item_id).bind(user_friendly_label).bind(existing_id)
                .execute(&mut **tx).await?;
            existing_id.to_owned()
        } else {
            query_scalar("INSERT INTO photo_cluster (id, user_id, thumbnail_media_item_id, centroid, friendly_label) VALUES ($1, $2, $3, $4, $5) RETURNING id")
                .bind(nice_id(10))
                .bind(user_id)
                .bind(thumbnail_media_item_id)
                .bind(&new_centroid)
                .bind(user_friendly_label)
                .fetch_one(&mut **tx).await?
        };

        query!("INSERT INTO media_item_photo_cluster (media_item_id, photo_cluster_id) SELECT unnest($1::varchar[]), $2 ON CONFLICT DO NOTHING", &media_item_ids, photo_cluster_id)
            .execute(&mut **tx).await?;
    }
    Ok(())
}

async fn cleanup_obsolete(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    existing_clusters: &[ExistingPhotoCluster],
    matched_ids: &HashSet<String>,
) -> Result<()> {
    let obsolete: Vec<String> = existing_clusters
        .iter()
        .filter(|c| !matched_ids.contains(&c.id))
        .map(|c| c.id.clone())
        .collect();
    if !obsolete.is_empty() {
        query!(
            "DELETE FROM media_item_photo_cluster WHERE photo_cluster_id = ANY($1)",
            &obsolete
        )
        .execute(&mut **tx)
        .await?;
        query!("DELETE FROM photo_cluster WHERE id = ANY($1)", &obsolete)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Checks if there have been any updates to media items since the last
/// successful photo cluster generation for the user.
async fn needs_clustering(pool: &PgPool, user_id: i32) -> Result<bool> {
    let needs_run = sqlx::query_scalar!(
        r#"
        WITH last_run AS (
            SELECT MAX(updated_at) AS last_run_time
            FROM photo_cluster
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
                        SELECT 1 FROM visual_analysis va
                        WHERE va.user_id = $1 AND va.created_at > (SELECT last_run_time FROM last_run)
                    )
                    OR EXISTS (
                        SELECT 1 FROM media_item mi
                        WHERE mi.user_id = $1 AND mi.updated_at > (SELECT last_run_time FROM last_run)
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

    // Load, resolve, and generate embeddings for cluster labels
    let now = Instant::now();
    let text_embedder = context
        .text_embedder
        .clone()
        .ok_or_else(|| eyre!("No text_embedder on worker that picked up cluster_photos job"))?;
    load_tag_embeddings(
        &context.pool,
        &text_embedder,
        &context.settings.ingest.analyzer.search.embedder_model_id,
    )
    .await?;
    info!("load_tag_embeddings took {:?}", now.elapsed());

    for user_id in user_ids {
        // Skip user if there are no new updates or changes since the last run
        if !needs_clustering(&context.pool, user_id).await? {
            info!(
                "Skipping photo clustering for user {} - no updates detected",
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
            .map(|p| p.embedding.to_vec())
            .collect();
        let (labels, new_centroids) =
            clustering::run_hdbscan(&embeddings, MIN_ITEMS_TO_CLUSTER, MIN_SAMPLES)?;

        let cluster_map = clustering::match_centroids(
            &new_centroids,
            &existing_clusters,
            CENTROID_MATCH_THRESHOLD,
        )?;
        let matched_old_ids: HashSet<String> = cluster_map.values().cloned().collect();
        let new_clusters = clustering::group_by_cluster(&labels, &items_to_cluster);

        let mut tx = context.pool.begin().await?;

        upsert_and_link(&mut tx, user_id, new_clusters, &new_centroids, &cluster_map).await?;
        cleanup_obsolete(&mut tx, &existing_clusters, &matched_old_ids).await?;

        tx.commit().await?;
        info!("Reconciled {} clusters for user {}", ENTITY_NAME, user_id);
    }

    Ok(JobResult::Done)
}
