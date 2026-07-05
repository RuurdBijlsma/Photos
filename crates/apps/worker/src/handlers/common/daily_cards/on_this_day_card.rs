use crate::handlers::common::daily_cards::DailyCardGenerator;
use app_state::AppSettings;
use async_trait::async_trait;
use chrono::Datelike;
use chrono::Utc;
use common_services::api::album::service::get_representative_thumbnail;
use rand::prelude::IndexedRandom;
use sqlx::PgTransaction;

pub struct OnThisDayCardGenerator;

#[async_trait]
#[allow(clippy::too_many_lines)]
impl DailyCardGenerator for OnThisDayCardGenerator {
    fn card_type(&self) -> &'static str {
        "on_this_day"
    }

    async fn generate(
        &self,
        tx: &mut PgTransaction<'_>,
        user_id: i32,
        _settings: &AppSettings,
    ) -> color_eyre::Result<()> {
        let today = Utc::now().naive_utc().date();

        for offset in 0..7 {
            let target_date = today + chrono::Duration::days(offset);

            // Check if card of type on_this_day already exists for this date
            let card_exists = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM daily_card WHERE user_id = $1 AND card_type = 'on_this_day' AND card_date = $2)",
                user_id,
                target_date
            )
                .fetch_one(&mut **tx)
                .await?
                .unwrap_or(false);

            if card_exists {
                continue;
            }

            let month = target_date.month() as i32;
            let day = target_date.day() as i32;
            let current_year = target_date.year();

            let available_years = sqlx::query_scalar!(
                r#"
                SELECT EXTRACT(YEAR FROM taken_at_local)::integer as "year!"
                FROM media_item
                WHERE user_id = $1 AND deleted = false
                  AND EXTRACT(MONTH FROM taken_at_local)::integer = $2
                  AND EXTRACT(DAY FROM taken_at_local)::integer = $3
                  AND EXTRACT(YEAR FROM taken_at_local)::integer < $4
                GROUP BY 1
                HAVING COUNT(*) >= 2
                "#,
                user_id,
                month,
                day,
                current_year
            )
            .fetch_all(&mut **tx)
            .await?;

            if available_years.is_empty() {
                continue;
            }

            let selected_year = {
                let mut rng = rand::rng();
                available_years.choose(&mut rng)
            };

            let Some(year) = selected_year else { continue };

            let items = sqlx::query!(
                r#"
                SELECT id, width, height, is_video, duration_ms, has_thumbnails, use_panorama_viewer, taken_at_local
                FROM media_item
                WHERE user_id = $1 AND deleted = false
                  AND EXTRACT(MONTH FROM taken_at_local)::integer = $2
                  AND EXTRACT(DAY FROM taken_at_local)::integer = $3
                  AND EXTRACT(YEAR FROM taken_at_local)::integer = $4
                ORDER BY sort_timestamp
                "#,
                user_id,
                month,
                day,
                year
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

            let diff_years = current_year - year;
            let title = "On This Day";
            let subtitle = format!("{diff_years} years ago");

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
                        "takenAtLocal": i.taken_at_local,
                    })
                })
                .collect();

            let payload = serde_json::json!({
                "mediaItems": payload_items
            });

            sqlx::query!(
                r#"
                INSERT INTO daily_card (user_id, card_type, card_date, title, subtitle, thumbnail_media_item_id, payload)
                VALUES ($1, 'on_this_day', $2, $3, $4, $5, $6)
                "#,
                user_id,
                target_date,
                title,
                subtitle,
                thumbnail_id,
                payload
            )
                .execute(&mut **tx)
                .await?;
        }

        Ok(())
    }
}
