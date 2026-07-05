use crate::handlers::common::daily_cards::DailyCardGenerator;
use app_state::AppSettings;
use async_trait::async_trait;
use common_services::api::album::service::get_representative_thumbnail;
use sqlx::PgTransaction;

pub struct ClusterCardGenerator;

#[async_trait]
impl DailyCardGenerator for ClusterCardGenerator {
    fn card_type(&self) -> &'static str {
        "cluster"
    }

    async fn generate(
        &self,
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        _settings: &AppSettings,
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
            "SELECT id, friendly_label FROM photo_cluster WHERE user_id = $1",
            user_id
        )
        .fetch_all(&mut **tx)
        .await?;

        for cluster in clusters {
            // Fetch media items in this cluster
            let items = sqlx::query!(
                r#"
                SELECT m.id, m.width, m.height, m.is_video, m.use_panorama_viewer, m.duration_ms, m.has_thumbnails
                FROM media_item_photo_cluster pc
                JOIN media_item m ON pc.media_item_id = m.id
                WHERE pc.photo_cluster_id = $1 AND m.deleted = false
                ORDER BY m.sort_timestamp
                "#,
                cluster.id
            )
                .fetch_all(&mut **tx)
                .await?;

            if items.is_empty() {
                continue;
            }

            let item_ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
            let thumbnail_id = get_representative_thumbnail(tx, &item_ids)
                .await
                .map_err(|e| {
                    color_eyre::eyre::eyre!("Failed to get representative thumbnail: {:?}", e)
                })?;

            let payload_items: Vec<serde_json::Value> = items
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
