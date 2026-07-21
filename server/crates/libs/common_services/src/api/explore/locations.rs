use crate::api::album::service::get_representative_thumbnail;
use crate::api::app_error::AppError;
use crate::api::explore::interfaces::VisitedLocation;
use common_types::pb::api::SimpleTimelineItem;
use sqlx::PgPool;

#[derive(Clone)]
struct RawLocation {
    id: i32,
    name: String,
    admin1: String,
    admin2: String,
    country_code: String,
    country_name: String,
    photo_count: i64,
}

/// Helper function to dynamically select the best representative thumbnail for a batch of locations.
async fn resolve_locations(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: i32,
    raw_locs: Vec<RawLocation>,
) -> Result<Vec<VisitedLocation>, AppError> {
    let mut resolved = Vec::with_capacity(raw_locs.len());
    for loc in raw_locs {
        // Fetch all non-deleted media item IDs for this specific location
        let media_item_ids = sqlx::query_scalar!(
            r#"
            SELECT m.id
            FROM media_item m
            JOIN gps g ON m.id = g.media_item_id
            WHERE m.user_id = $1 AND g.location_id = $2 AND m.deleted = false
            "#,
            user_id,
            loc.id
        )
        .fetch_all(&mut **tx)
        .await?;
        let thumbnail_id = get_representative_thumbnail(tx, &media_item_ids).await?;

        resolved.push(VisitedLocation {
            id: loc.id,
            name: loc.name,
            admin1: loc.admin1,
            admin2: loc.admin2,
            country_code: loc.country_code,
            country_name: loc.country_name,
            photo_count: loc.photo_count,
            thumbnail_id,
        });
    }
    Ok(resolved)
}

pub async fn get_visited_places(
    pool: &PgPool,
    user_id: i32,
) -> Result<Vec<VisitedLocation>, AppError> {
    // todo: i'm sure this entire func can be made more performant
    let mut tx = pool.begin().await?;

    // 1. Query Top 30 locations in a single call (optimizes landmarks & past adventures)
    // todo: this query is weird, why not query_as
    let top_locations_rows = sqlx::query!(
        r#"
        SELECT l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name, COUNT(*)::bigint as "photo_count!"
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        JOIN location l ON g.location_id = l.id
        WHERE m.user_id = $1 AND m.deleted = false
        GROUP BY l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name
        ORDER BY "photo_count!" DESC, l.id DESC
        LIMIT 30
        "#,
        user_id
    )
        .fetch_all(&mut *tx)
        .await?;

    // Map immediately to owned, cloneable RawLocation objects
    let top_locations: Vec<RawLocation> = top_locations_rows
        .into_iter()
        .map(|r| RawLocation {
            id: r.id,
            name: r.name,
            admin1: r.admin1,
            admin2: r.admin2,
            country_code: r.country_code,
            country_name: r.country_name,
            photo_count: r.photo_count,
        })
        .collect();

    // Top Landmarks: Randomly sample 3 from ranks 1 to 10
    let top_10: Vec<RawLocation> = top_locations.iter().take(10).cloned().collect();
    let mut top_landmarks_raw = Vec::new();
    if !top_10.is_empty() {
        let mut indices: Vec<usize> = (0..top_10.len()).collect();
        fastrand::shuffle(&mut indices);
        for &idx in indices.iter().take(3) {
            if let Some(loc) = top_10.get(idx) {
                top_landmarks_raw.push(loc.clone());
            }
        }
    }

    // Past Adventures: Randomly sample 3 from ranks 11 to 30
    let ranks_11_30: Vec<RawLocation> = top_locations.iter().skip(10).take(20).cloned().collect();
    let mut past_adventures_raw = Vec::new();
    if !ranks_11_30.is_empty() {
        let mut indices: Vec<usize> = (0..ranks_11_30.len()).collect();
        fastrand::shuffle(&mut indices);
        for &idx in indices.iter().take(3) {
            if let Some(loc) = ranks_11_30.get(idx) {
                past_adventures_raw.push(loc.clone());
            }
        }
    }

    // 2. Query Hidden Gems: Counts between 5 and 20, sampled down from a broader pool
    // todo: also weird query
    let hidden_gems_rows = sqlx::query!(
        r#"
        SELECT l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name, COUNT(*)::bigint as "photo_count!"
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        JOIN location l ON g.location_id = l.id
        WHERE m.user_id = $1 AND m.deleted = false
        GROUP BY l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name
        HAVING COUNT(*) >= 5 AND COUNT(*) <= 20
        ORDER BY COUNT(*) DESC, l.id DESC
        LIMIT 50
        "#,
        user_id
    )
        .fetch_all(&mut *tx)
        .await?;

    let hidden_gems_all: Vec<RawLocation> = hidden_gems_rows
        .into_iter()
        .map(|r| RawLocation {
            id: r.id,
            name: r.name,
            admin1: r.admin1,
            admin2: r.admin2,
            country_code: r.country_code,
            country_name: r.country_name,
            photo_count: r.photo_count,
        })
        .collect();

    let mut hidden_gems_raw = Vec::new();
    if !hidden_gems_all.is_empty() {
        let mut indices: Vec<usize> = (0..hidden_gems_all.len()).collect();
        fastrand::shuffle(&mut indices);
        for &idx in indices.iter().take(3) {
            if let Some(loc) = hidden_gems_all.get(idx) {
                hidden_gems_raw.push(loc.clone());
            }
        }
    }

    // 3. Query Recent Destinations
    // Sorts places, recent first, based on the oldest media item associated with each place.
    let recent_destinations_rows = sqlx::query!(
        r#"
        SELECT
            l.id,
            l.name,
            l.admin1,
            l.admin2,
            l.country_code,
            l.country_name,
            COUNT(*)::bigint as "photo_count!"
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        JOIN location l ON g.location_id = l.id
        WHERE m.user_id = $1 AND m.deleted = false
        GROUP BY l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name
        HAVING COUNT(*) >= 3
        ORDER BY MIN(m.sort_timestamp) DESC, l.id DESC
        LIMIT 3
        "#,
        user_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let recent_destinations_raw: Vec<RawLocation> = recent_destinations_rows
        .into_iter()
        .map(|r| RawLocation {
            id: r.id,
            name: r.name,
            admin1: r.admin1,
            admin2: r.admin2,
            country_code: r.country_code,
            country_name: r.country_name,
            photo_count: r.photo_count,
        })
        .collect();

    // Resolve representative thumbnails for each category
    let top_landmarks = resolve_locations(&mut tx, user_id, top_landmarks_raw).await?;
    let past_adventures = resolve_locations(&mut tx, user_id, past_adventures_raw).await?;
    let hidden_gems = resolve_locations(&mut tx, user_id, hidden_gems_raw).await?;
    let recent_destinations = resolve_locations(&mut tx, user_id, recent_destinations_raw).await?;

    tx.commit().await?;

    // Combine and deduplicate
    let mut all_locations = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for loc in top_landmarks
        .into_iter()
        .chain(past_adventures.into_iter())
        .chain(hidden_gems.into_iter())
        .chain(recent_destinations.into_iter())
    {
        if seen_ids.insert(loc.id) {
            all_locations.push(loc);
        }
    }

    // Shuffle the combined list to mix the categories
    fastrand::shuffle(&mut all_locations);

    Ok(all_locations)
}

/// Retrieve chronological `SimpleTimelineItem` protobuf records matching a selected location
pub async fn get_location_media(
    pool: &PgPool,
    user_id: i32,
    location_id: i32,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            m.id,
            m.is_video as "is_video!",
            m.has_thumbnails as "has_thumbnails!",
            m.duration_ms,
            m.width,
            m.height
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        WHERE m.user_id = $1 AND g.location_id = $2 AND m.deleted = false
        ORDER BY m.sort_timestamp, id
        "#,
        user_id,
        location_id
    )
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let ratio = if row.height > 0 {
                row.width as f32 / row.height as f32
            } else {
                1.0
            };
            SimpleTimelineItem {
                id: row.id,
                is_video: row.is_video,
                has_thumbnails: row.has_thumbnails,
                duration_ms: row.duration_ms.map(|d| d as i32),
                ratio,
            }
        })
        .collect();

    Ok(items)
}

/// Simple utility to fetch isolated metadata details for a location header
pub async fn get_location_details(
    pool: &PgPool,
    user_id: i32,
    location_id: i32,
) -> Result<VisitedLocation, AppError> {
    let mut tx = pool.begin().await?;

    let loc = sqlx::query!(
        r#"
        SELECT l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name, COUNT(*)::bigint as "photo_count!"
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        JOIN location l ON g.location_id = l.id
        WHERE m.user_id = $1 AND l.id = $2 AND m.deleted = false
        GROUP BY l.id, l.name, l.admin1, l.admin2, l.country_code, l.country_name
        "#,
        user_id,
        location_id
    )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Location {location_id} not found")))?;

    let media_item_ids = sqlx::query_scalar!(
        r#"
        SELECT m.id
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        WHERE m.user_id = $1 AND g.location_id = $2 AND m.deleted = false
        "#,
        user_id,
        location_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let thumbnail_id = get_representative_thumbnail(&mut tx, &media_item_ids).await?;

    tx.commit().await?;

    Ok(VisitedLocation {
        id: loc.id,
        name: loc.name,
        admin1: loc.admin1,
        admin2: loc.admin2,
        country_code: loc.country_code,
        country_name: loc.country_name,
        photo_count: loc.photo_count,
        thumbnail_id,
    })
}
