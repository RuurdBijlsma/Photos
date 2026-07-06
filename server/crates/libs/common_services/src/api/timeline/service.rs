use crate::api::app_error::AppError;
use crate::api::timeline::interfaces::SortDirection;
use crate::database::app_user::User;
use chrono::NaiveDate;
use common_types::pb::api::{
    TimelineItem, TimelineItemsResponse, TimelineMonthItems, TimelineMonthRatios,
    TimelineRatiosResponse,
};
use sqlx::PgPool;
use std::collections::HashMap;

/// Fetches a timeline of media item ratios, grouped by month.
pub async fn get_timeline_ratios(
    user: &User,
    pool: &PgPool,
    sort_direction: SortDirection,
) -> Result<TimelineRatiosResponse, AppError> {
    let sql = format!(
        r"
        SELECT
            month_id::TEXT as month_id,
            COUNT(*)::INT AS count,
            array_agg(width::real / height::real ORDER BY sort_timestamp {0}) AS ratios
        FROM media_item
        WHERE user_id = $1
          AND deleted = false
        GROUP BY month_id
        ORDER BY month_id {0}
        ",
        sort_direction.as_sql()
    );

    let months = sqlx::query_as::<_, TimelineMonthRatios>(&sql)
        .bind(user.id)
        .fetch_all(pool)
        .await?;

    Ok(TimelineRatiosResponse { months })
}

/// Fetches a timeline of media item ids.
pub async fn get_timeline_ids(
    user: &User,
    pool: &PgPool,
    sort_direction: SortDirection,
) -> Result<Vec<String>, AppError> {
    let sql = format!(
        r"
        SELECT COALESCE(array_agg(id ORDER BY sort_timestamp {}), '{{}}')
        FROM media_item
        WHERE user_id = $1 AND deleted = false
        ",
        sort_direction.as_sql()
    );

    let ids = sqlx::query_scalar::<_, Vec<String>>(&sql)
        .bind(user.id)
        .fetch_one(pool)
        .await?;

    Ok(ids)
}

/// Fetches media items for a given list of month IDs, grouped by month.
pub async fn get_photos_by_month(
    user: &User,
    pool: &PgPool,
    month_ids: &[NaiveDate],
    sort_direction: SortDirection,
) -> Result<TimelineItemsResponse, AppError> {
    let sql = format!(
        r"
        SELECT
            id,
            is_video,
            has_thumbnails,
            duration_ms::INT as duration_ms,
            taken_at_local::TEXT as timestamp
        FROM
            media_item
        WHERE
            user_id = $1
            AND deleted = false
            AND month_id = ANY($2)
        ORDER BY
            sort_timestamp {}
        ",
        sort_direction.as_sql()
    );

    let items = sqlx::query_as::<_, TimelineItem>(&sql)
        .bind(user.id)
        .bind(month_ids)
        .fetch_all(pool)
        .await?;

    // Grouping logic
    let mut months_map: HashMap<String, Vec<TimelineItem>> = HashMap::new();
    for item in items {
        // Assuming timestamp format YYYY-MM-DD...
        if item.timestamp.len() >= 7 {
            let month_id = format!("{}-01", &item.timestamp[0..7]);
            months_map.entry(month_id).or_default().push(item);
        }
    }

    let months = months_map
        .into_iter()
        .map(|(month_id, items)| TimelineMonthItems { month_id, items })
        .collect();

    Ok(TimelineItemsResponse { months })
}
