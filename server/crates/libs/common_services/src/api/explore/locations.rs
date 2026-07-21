use crate::api::album::service::get_representative_thumbnail;
use crate::api::app_error::AppError;
use crate::api::explore::interfaces::VisitedLocation;
use common_types::pb::api::SimpleTimelineItem;
use sqlx::PgPool;
use std::collections::HashMap;

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
    if raw_locs.is_empty() {
        return Ok(Vec::new());
    }

    let loc_ids: Vec<i32> = raw_locs.iter().map(|l| l.id).collect();

    // Batch fetch all media items for the locations
    let mappings = sqlx::query!(
        r#"
        SELECT g.location_id as "location_id!", m.id as "media_id!"
        FROM media_item m
        JOIN gps g ON m.id = g.media_item_id
        WHERE m.user_id = $1 AND g.location_id = ANY($2) AND m.deleted = false
        "#,
        user_id,
        &loc_ids
    )
        .fetch_all(&mut **tx)
        .await?;

    // Group the media items by their location_id
    let mut loc_to_media: HashMap<i32, Vec<String>> = HashMap::new();
    for row in mappings {
        loc_to_media.entry(row.location_id).or_default().push(row.media_id);
    }

    let mut resolved = Vec::with_capacity(raw_locs.len());
    for loc in raw_locs {
        let media_item_ids = loc_to_media.get(&loc.id).cloned().unwrap_or_default();
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

    let mut top_10: Vec<RawLocation> = top_locations.iter().take(10).cloned().collect();
    fastrand::shuffle(&mut top_10);
    let common_locs_raw: Vec<RawLocation> = top_10.into_iter().take(3).collect();

    let mut ranks_11_30: Vec<RawLocation> = top_locations.iter().skip(10).take(20).cloned().collect();
    fastrand::shuffle(&mut ranks_11_30);
    let rare_locs_raw: Vec<RawLocation> = ranks_11_30.into_iter().take(3).collect();

    let legendary_locs_rows = sqlx::query!(
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

    let mut legendary_locs_all: Vec<RawLocation> = legendary_locs_rows
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

    fastrand::shuffle(&mut legendary_locs_all);
    let legendary_locs_raw: Vec<RawLocation> = legendary_locs_all.into_iter().take(3).collect();

    // Retrieving Recent Destinations
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

    let common_locs = resolve_locations(&mut tx, user_id, common_locs_raw).await?;
    let rare_locs = resolve_locations(&mut tx, user_id, rare_locs_raw).await?;
    let legendary_locs = resolve_locations(&mut tx, user_id, legendary_locs_raw).await?;
    let recent_destinations = resolve_locations(&mut tx, user_id, recent_destinations_raw).await?;

    tx.commit().await?;

    let mut all_locations = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for loc in common_locs
        .into_iter()
        .chain(rare_locs.into_iter())
        .chain(legendary_locs.into_iter())
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
            let rows = sqlx::query!(
                r#"
                SELECT m.id as "media_item_id!", l.name, l.admin1, l.admin2, l.country_code, l.country_name
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.id = $2 AND m.deleted = false
                "#,
                user_id,
                id
            )
                .fetch_all(&mut *tx)
                .await?;

            if rows.is_empty() {
                return Err(AppError::NotFound(format!("Location {id} not found")));
            }

            let first = &rows[0];
            let name = first.name.clone();
            let admin1 = first.admin1.clone();
            let admin2 = first.admin2.clone();
            let country_code = first.country_code.clone();
            let country_name = first.country_name.clone();
            let photo_count = rows.len() as i64;
            let media_item_ids: Vec<String> = rows.into_iter().map(|r| r.media_item_id).collect();

            (name, admin1, admin2, country_code, country_name, photo_count, media_item_ids)
        }
        LocationScope::Country(code) => {
            let rows = sqlx::query!(
                r#"
                SELECT m.id as "media_item_id!", l.country_name
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND m.deleted = false
                "#,
                user_id,
                &code
            )
                .fetch_all(&mut *tx)
                .await?;

            if rows.is_empty() {
                return Err(AppError::NotFound(format!("Country {code} not found")));
            }

            let country_name = rows[0].country_name.clone();
            let photo_count = rows.len() as i64;
            let media_item_ids: Vec<String> = rows.into_iter().map(|r| r.media_item_id).collect();

            (country_name.clone(), String::new(), String::new(), code, country_name, photo_count, media_item_ids)
        }
        LocationScope::Admin1 { country_code, admin1 } => {
            let rows = sqlx::query!(
                r#"
                SELECT m.id as "media_item_id!", l.country_name
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND l.admin1 = $3 AND m.deleted = false
                "#,
                user_id,
                &country_code,
                &admin1
            )
                .fetch_all(&mut *tx)
                .await?;

            if rows.is_empty() {
                return Err(AppError::NotFound(format!("Region {admin1} in {country_code} not found")));
            }

            let country_name = rows[0].country_name.clone();
            let photo_count = rows.len() as i64;
            let media_item_ids: Vec<String> = rows.into_iter().map(|r| r.media_item_id).collect();

            (admin1.clone(), admin1, String::new(), country_code, country_name, photo_count, media_item_ids)
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