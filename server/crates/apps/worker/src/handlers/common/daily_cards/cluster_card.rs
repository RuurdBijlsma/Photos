use crate::handlers::common::clustering::{self, CandidatePhoto};
use crate::handlers::common::daily_cards::DailyCardGenerator;
use app_state::AppSettings;
use async_trait::async_trait;
use pgvector::Vector;
use sqlx::PgTransaction;

pub struct ClusterCardGenerator;

#[async_trait]
#[allow(clippy::too_many_lines)]
impl DailyCardGenerator for ClusterCardGenerator {
    fn card_type(&self) -> &'static str {
        "cluster"
    }

    async fn generate(
        &self,
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        settings: &AppSettings,
    ) -> color_eyre::Result<()> {
        let latest_cluster_update: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar!(
            "SELECT MAX(updated_at) FROM photo_cluster WHERE user_id = $1",
            user_id
        )
        .fetch_one(&mut **tx)
        .await?;

        let latest_card_creation: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar!(
            "SELECT MAX(created_at) FROM daily_card WHERE user_id = $1 AND card_type = 'cluster'",
            user_id
        )
        .fetch_one(&mut **tx)
        .await?;

        let needs_regeneration = match (latest_cluster_update, latest_card_creation) {
            (Some(upd), Some(cre)) => upd > cre,
            (Some(_), None) => true,
            _ => false,
        };

        if !needs_regeneration {
            return Ok(());
        }

        // Delete all cluster daily_cards for this user
        sqlx::query!(
            "DELETE FROM daily_card WHERE user_id = $1 AND card_type = 'cluster'",
            user_id
        )
        .execute(&mut **tx)
        .await?;

        // Fetch all clusters for the user
        let clusters = sqlx::query!(
            r#"
            SELECT id, friendly_label, centroid as "centroid: Vector"
            FROM photo_cluster
            WHERE user_id = $1
            ORDER BY updated_at DESC
            "#,
            user_id
        )
        .fetch_all(&mut **tx)
        .await?;

        let clustering_settings = settings.ingest.analyzer.clustering;

        for cluster in clusters {
            // Fetch media items in this cluster along with session_id and embedding
            let items = sqlx::query!(
                r#"
                SELECT
                    m.id,
                    m.width,
                    m.height,
                    m.is_video,
                    m.use_panorama_viewer,
                    m.duration_ms,
                    m.has_thumbnails,
                    m.sort_timestamp,
                    pc.session_id,
                    va.embedding as "embedding!: Vector"
                FROM media_item_photo_cluster pc
                JOIN media_item m ON pc.media_item_id = m.id
                JOIN LATERAL (
                    SELECT va.embedding
                    FROM visual_analysis va
                    WHERE va.media_item_id = m.id AND va.deleted = false
                    ORDER BY va.created_at DESC
                    LIMIT 1
                ) va ON true
                WHERE pc.photo_cluster_id = $1 AND m.deleted = false
                ORDER BY m.sort_timestamp ASC
                "#,
                cluster.id
            )
            .fetch_all(&mut **tx)
            .await?;

            if items.is_empty() {
                continue;
            }

            // Prepare candidates for MMR diversity sampling
            let candidates: Vec<CandidatePhoto> = items
                .iter()
                .map(|i| CandidatePhoto {
                    media_item_id: i.id.clone(),
                    session_id: i.session_id,
                    embedding: i.embedding.to_vec(),
                })
                .collect();

            // Determine cluster centroid
            let centroid: Vec<f32> = cluster.centroid.map_or_else(
                || {
                    let mut avg = vec![0.0f32; candidates[0].embedding.len()];
                    for c in &candidates {
                        for (k, &val) in c.embedding.iter().enumerate() {
                            if k < avg.len() {
                                avg[k] += val;
                            }
                        }
                    }
                    clustering::normalize_vector(&mut avg);
                    avg
                },
                |c| c.to_vec(),
            );

            let selected_indices = clustering::select_mmr_photos(
                &candidates,
                &centroid,
                clustering_settings.card_max_photos,
                clustering_settings.mmr_relevance_lambda,
            );

            if selected_indices.len() < clustering_settings.card_min_photos {
                continue;
            }

            // The first MMR pick is the most representative photo of the theme centroid
            let thumbnail_id = items[selected_indices[0]].id.clone();

            // Order selected photos chronologically for the card slideshow
            let mut selected_items: Vec<_> = selected_indices
                .into_iter()
                .map(|idx| &items[idx])
                .collect();
            selected_items.sort_by_key(|i| i.sort_timestamp);

            let payload_items: Vec<serde_json::Value> = selected_items
                .iter()
                .map(|i| {
                    serde_json::json!({
                        "id": i.id,
                        "ratio": f64::from(i.width) / f64::from(i.height),
                        "durationMs": i.duration_ms,
                        "hasThumbnails": i.has_thumbnails,
                        "isVideo": i.is_video,
                        "width": i.width,
                        "height": i.height,
                        "usePanoramaViewer": i.use_panorama_viewer,
                    })
                })
                .collect();

            let payload = serde_json::json!({
                "mediaItems": payload_items
            });

            sqlx::query!(
                r#"
                INSERT INTO daily_card (user_id, card_type, title, thumbnail_media_item_id, payload)
                VALUES ($1, 'cluster', $2, $3, $4)
                "#,
                user_id,
                cluster
                    .friendly_label
                    .unwrap_or_else(|| "Cluster".to_string()),
                thumbnail_id,
                payload
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }
}
