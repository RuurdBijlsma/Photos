use crate::api::app_error::AppError;
use crate::api::daily_cards::interfaces::DailyCardResponse;
use app_state::AppSettings;
use chrono::NaiveDate;
use rand::RngExt;
use sqlx::PgPool;

pub async fn get_daily_cards(
    pool: &PgPool,
    user_id: i32,
    target_date: NaiveDate,
    settings: &AppSettings,
) -> Result<Vec<DailyCardResponse>, AppError> {
    let mut tx = pool.begin().await?;

    let mut returned_cards = Vec::new();

    // Fetch date-bound cards for the given date
    let date_cards = sqlx::query_as!(
        DailyCardResponse,
        r#"
        SELECT id, card_date, card_type, title, subtitle, thumbnail_media_item_id, payload
        FROM daily_card
        WHERE user_id = $1 AND card_date = $2
        "#,
        user_id,
        target_date
    )
    .fetch_all(&mut *tx)
    .await?;

    returned_cards.extend(date_cards);

    // Fetch non-date cards where shown = false
    let types: Vec<String> = sqlx::query_scalar!(
        "SELECT DISTINCT card_type FROM daily_card WHERE user_id = $1 AND card_date IS NULL AND shown = false",
        user_id
    )
        .fetch_all(&mut *tx)
        .await?;

    for card_type in types {
        let (min, max) = match card_type.as_str() {
            "cluster" => (
                settings.daily_cards.cluster.min_cards,
                settings.daily_cards.cluster.max_cards,
            ),
            _ => (1, 1),
        };

        let count = if min >= max {
            min
        } else {
            rand::rng().random_range(min..=max)
        };

        if count > 0 {
            let type_cards = sqlx::query_as!(
                DailyCardResponse,
                r#"
                SELECT id, card_date, card_type, title, subtitle, thumbnail_media_item_id, payload
                FROM daily_card
                WHERE user_id = $1 AND card_type = $2 AND card_date IS NULL AND shown = false
                ORDER BY created_at
                LIMIT $3
                "#,
                user_id,
                card_type,
                count as i64
            )
            .fetch_all(&mut *tx)
            .await?;

            returned_cards.extend(type_cards);
        }
    }

    // Sort returned cards:
    // 1. on this day
    // 2. location estimatr
    // 3. the rest
    returned_cards.sort_by_key(|card| match card.card_type.as_str() {
        "on_this_day" => 0,
        "estimatr" => 1,
        _ => 2,
    });

    // Mark all returned cards as shown = true
    if !returned_cards.is_empty() {
        let ids: Vec<i32> = returned_cards.iter().map(|c| c.id).collect();
        sqlx::query!(
            "UPDATE daily_card SET shown = true, updated_at = now() WHERE id = ANY($1)",
            &ids
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(returned_cards)
}

pub async fn validate_media_items(
    pool: &PgPool,
    user_id: i32,
    media_item_ids: &[String],
) -> Result<Vec<String>, AppError> {
    let active_ids = sqlx::query_scalar!(
        "SELECT id FROM media_item WHERE user_id = $1 AND id = ANY($2) AND deleted = false",
        user_id,
        media_item_ids
    )
    .fetch_all(pool)
    .await?;

    Ok(active_ids)
}
