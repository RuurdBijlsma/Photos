use crate::handlers::common::daily_cards::DailyCardGenerator;
use app_state::AppSettings;
use async_trait::async_trait;
use common_services::api::album::service::get_representative_thumbnail;
use common_services::database::key_json_store::KeyJsonStore;
use sqlx::PgTransaction;

pub struct LocationEstimatrCardGenerator;

#[async_trait]
#[allow(clippy::too_many_lines)]
impl DailyCardGenerator for LocationEstimatrCardGenerator {
    fn card_type(&self) -> &'static str {
        "estimatr"
    }

    async fn generate(
        &self,
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        settings: &AppSettings,
    ) -> color_eyre::Result<()> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM daily_card WHERE user_id = $1 AND card_type = 'estimatr' AND shown = false",
            user_id
        )
            .fetch_one(&mut **tx)
            .await?
            .unwrap_or(0);

        if count >= 7 {
            return Ok(());
        }

        let cards_to_generate = 7 - count;
        let limit = settings.daily_cards.estimatr.rounds_per_day;

        // Fetch user most frequent location and calculated total area
        let most_frequent_location =
            KeyJsonStore::get_value(&mut **tx, "most_frequent_location", Some(user_id)).await?;
        let total_area_sq_km =
            KeyJsonStore::get_value(&mut **tx, "total_area_sq_km", Some(user_id)).await?;

        let (home_lat, home_lon, exclusion_radius) = if let Some(loc_val) = most_frequent_location
            && let Some(lat) = loc_val.get("latitude").and_then(serde_json::Value::as_f64)
            && let Some(lon) = loc_val.get("longitude").and_then(serde_json::Value::as_f64)
        {
            (lat, lon, 10.0) // 10 km exclusion zone
        } else {
            (0.0, 0.0, 0.0)
        };

        let area_km2 = total_area_sq_km
            .and_then(|v| v.as_f64())
            .filter(|&v| v > 0.0)
            .unwrap_or(10_530_000.0); // Fallback to a default size if not calculated

        for _ in 0..cards_to_generate {
            // Select random media items with valid GPS, prioritizing those outside the exclusion zone.
            let mut items = sqlx::query!(
                r#"
                SELECT m.id, m.width, m.height, m.is_video, m.use_panorama_viewer,
                       g.latitude, g.longitude, m.duration_ms, m.taken_at_local, m.has_thumbnails
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                WHERE m.user_id = $1 AND m.deleted = false
                  AND g.latitude != 0.0 AND g.longitude != 0.0
                ORDER BY
                  (CASE
                    WHEN $4::double precision = 0.0 THEN 1
                    WHEN (sqrt(power((g.longitude - $3::double precision) * cos(radians($2::double precision)), 2) + power(g.latitude - $2::double precision, 2)) * 111.32) > $4::double precision THEN 1
                    ELSE 0
                  END) DESC,
                  random()
                LIMIT $5
                "#,
                user_id,
                home_lat,
                home_lon,
                exclusion_radius,
                limit
            )
                .fetch_all(&mut **tx)
                .await?;

            if items.is_empty() {
                break;
            }

            let item_ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();
            let thumbnail_id = get_representative_thumbnail(tx, &item_ids)
                .await
                .map_err(|e| {
                    color_eyre::eyre::eyre!("Failed to get representative thumbnail: {:?}", e)
                })?;

            // Rearrange `items` so the thumbnail item is at index 0
            if let Some(ref thumb_id) = thumbnail_id
                && let Some(pos) = items.iter().position(|i| &i.id == thumb_id)
            {
                let item = items.remove(pos);
                items.insert(0, item);
            }

            let rounds: Vec<serde_json::Value> = items
                .iter()
                .map(|i| {
                    serde_json::json!({
                        "mediaItem": {
                            "id": i.id,
                            "ratio": f64::from(i.width) / f64::from(i.height),
                            "durationMs": i.duration_ms,
                            "hasThumbnails": i.has_thumbnails,
                            "isVideo": i.is_video,
                            "width": i.width,
                            "height": i.height,
                            "usePanoramaViewer": i.use_panorama_viewer,
                            "takenAtLocal": i.taken_at_local,
                        },
                        "latitude": i.latitude,
                        "longitude": i.longitude,
                    })
                })
                .collect();

            let payload = serde_json::json!({
                "rounds": rounds,
                "areaKm2": area_km2,
            });

            sqlx::query!(
                r#"
                INSERT INTO daily_card (user_id, card_type, title, subtitle, thumbnail_media_item_id, payload)
                VALUES ($1, 'estimatr', 'Location Estimatr', 'Where was this taken?', $2, $3)
                "#,
                user_id,
                thumbnail_id,
                payload
            )
                .execute(&mut **tx)
                .await?;
        }

        Ok(())
    }
}
