use crate::api::album::service::get_representative_thumbnail;
use crate::api::app_error::AppError;
use crate::api::explore::interfaces::VisitedLocation;
use common_types::pb::api::SimpleTimelineItem;
use sqlx::PgPool;

pub enum LocationScope {
    Place(i32),
    Country(String),
    Admin1 { country_code: String, admin1: String },
}

impl LocationScope {
    pub fn parse(key: &str) -> Result<Self, AppError> {
        if let Some(rest) = key.strip_prefix("place:") {
            let id = rest.parse::<i32>().map_err(|_| {
                AppError::BadRequest("Invalid place ID format".to_owned())
            })?;
            Ok(Self::Place(id))
        } else if let Some(rest) = key.strip_prefix("country:") {
            Ok(Self::Country(rest.to_string()))
        } else if let Some(rest) = key.strip_prefix("admin1:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(AppError::BadRequest(
                    "Invalid admin1 key. Expected admin1:country_code:name".to_owned(),
                ));
            }
            Ok(Self::Admin1 {
                country_code: parts[0].to_string(),
                admin1: parts[1].to_string(),
            })
        } else {
            // Backward-compatible fallback for plain numerical IDs
            let id = key.parse::<i32>().map_err(|_| {
                AppError::BadRequest("Invalid numeric location ID".to_owned())
            })?;
            Ok(Self::Place(id))
        }
    }
}

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

async fn resolve_locations(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: i32,
    raw_locs: Vec<RawLocation>,
) -> Result<Vec<VisitedLocation>, AppError> {
    let mut resolved = Vec::with_capacity(raw_locs.len());
    for loc in raw_locs {
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
            id: format!("place:{}", loc.id),
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
    let mut tx = pool.begin().await?;

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

    let top_landmarks = resolve_locations(&mut tx, user_id, top_landmarks_raw).await?;
    let past_adventures = resolve_locations(&mut tx, user_id, past_adventures_raw).await?;
    let hidden_gems = resolve_locations(&mut tx, user_id, hidden_gems_raw).await?;
    let recent_destinations = resolve_locations(&mut tx, user_id, recent_destinations_raw).await?;

    tx.commit().await?;

    let mut all_locations = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for loc in top_landmarks
        .into_iter()
        .chain(past_adventures.into_iter())
        .chain(hidden_gems.into_iter())
        .chain(recent_destinations.into_iter())
    {
        if seen_ids.insert(loc.id.clone()) {
            all_locations.push(loc);
        }
    }

    fastrand::shuffle(&mut all_locations);
    Ok(all_locations)
}

/// Retrieve chronological `SimpleTimelineItem` protobuf records matching a selected location
pub async fn get_location_media(
    pool: &PgPool,
    user_id: i32,
    location_key: &str,
) -> Result<Vec<SimpleTimelineItem>, AppError> {
    let scope = LocationScope::parse(location_key)?;

    let rows = match scope {
        LocationScope::Place(id) => {
            sqlx::query_as!(
                SimpleTimelineItem,
                r#"
                SELECT
                    m.id,
                    m.is_video as "is_video!",
                    m.has_thumbnails as "has_thumbnails!",
                    m.duration_ms::INT,
                    (m.width::real / m.height::real) AS "ratio!"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                WHERE m.user_id = $1 AND g.location_id = $2 AND m.deleted = false
                ORDER BY m.sort_timestamp, id
                "#,
                user_id,
                id
            )
                .fetch_all(pool)
                .await?
        }
        LocationScope::Country(code) => {
            sqlx::query_as!(
                SimpleTimelineItem,
                r#"
                SELECT
                    m.id,
                    m.is_video as "is_video!",
                    m.has_thumbnails as "has_thumbnails!",
                    m.duration_ms::INT,
                    (m.width::real / m.height::real) AS "ratio!"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND m.deleted = false
                ORDER BY m.sort_timestamp, id
                "#,
                user_id,
                code
            )
                .fetch_all(pool)
                .await?
        }
        LocationScope::Admin1 { country_code, admin1 } => {
            sqlx::query_as!(
                SimpleTimelineItem,
                r#"
                SELECT
                    m.id,
                    m.is_video as "is_video!",
                    m.has_thumbnails as "has_thumbnails!",
                    m.duration_ms::INT,
                    (m.width::real / m.height::real) AS "ratio!"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND l.admin1 = $3 AND m.deleted = false
                ORDER BY m.sort_timestamp, id
                "#,
                user_id,
                country_code,
                admin1
            )
                .fetch_all(pool)
                .await?
        }
    };

    Ok(rows)
}

pub async fn get_location_details(
    pool: &PgPool,
    user_id: i32,
    location_key: &str,
) -> Result<VisitedLocation, AppError> {
    let scope = LocationScope::parse(location_key)?;
    let mut tx = pool.begin().await?;

    let (loc_name, loc_admin1, loc_admin2, loc_country_code, loc_country_name, photo_count, media_item_ids) = match scope {
        LocationScope::Place(id) => {
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
                id
            )
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Location {id} not found")))?;

            let media_item_ids = sqlx::query_scalar!(
                r#"
                SELECT m.id
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                WHERE m.user_id = $1 AND g.location_id = $2 AND m.deleted = false
                "#,
                user_id,
                id
            )
                .fetch_all(&mut *tx)
                .await?;

            (loc.name, loc.admin1, loc.admin2, loc.country_code, loc.country_name, loc.photo_count, media_item_ids)
        }
        LocationScope::Country(code) => {
            let loc = sqlx::query!(
                r#"
                SELECT l.country_name, COUNT(*)::bigint as "photo_count!"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND m.deleted = false
                GROUP BY l.country_name
                "#,
                user_id,
                code
            )
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Country {code} not found")))?;

            let media_item_ids = sqlx::query_scalar!(
                r#"
                SELECT m.id
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND m.deleted = false
                "#,
                user_id,
                code
            )
                .fetch_all(&mut *tx)
                .await?;

            (loc.country_name.clone(), String::new(), String::new(), code, loc.country_name, loc.photo_count, media_item_ids)
        }
        LocationScope::Admin1 { country_code, admin1 } => {
            let loc = sqlx::query!(
                r#"
                SELECT l.country_name, COUNT(*)::bigint as "photo_count!"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND l.admin1 = $3 AND m.deleted = false
                GROUP BY l.country_name
                "#,
                user_id,
                country_code,
                admin1
            )
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Region {admin1} in {country_code} not found")))?;

            let media_item_ids = sqlx::query_scalar!(
                r#"
                SELECT m.id
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND l.admin1 = $3 AND m.deleted = false
                "#,
                user_id,
                country_code,
                admin1
            )
                .fetch_all(&mut *tx)
                .await?;

            (admin1.clone(), admin1, String::new(), country_code, loc.country_name, loc.photo_count, media_item_ids)
        }
    };

    let thumbnail_id = get_representative_thumbnail(&mut tx, &media_item_ids).await?;
    tx.commit().await?;

    Ok(VisitedLocation {
        id: location_key.to_string(),
        name: loc_name,
        admin1: loc_admin1,
        admin2: loc_admin2,
        country_code: loc_country_code,
        country_name: loc_country_name,
        photo_count,
        thumbnail_id,
    })
}