use crate::api::album::service::get_representative_thumbnail;
use crate::api::app_error::AppError;
use common_types::pb::api::{LocationDetailsResponse, LocationMediaItem, VisitedLocation};
use sqlx::{Executor, PgPool, Postgres};
use std::collections::HashMap;

pub enum LocationScope {
    Place(i32),
    Country(String),
    Admin1 {
        country_code: String,
        admin1: String,
    },
}

impl LocationScope {
    pub fn parse(key: &str) -> Result<Self, AppError> {
        if let Some(rest) = key.strip_prefix("place:") {
            let id = rest
                .parse::<i32>()
                .map_err(|_| AppError::BadRequest("Invalid place ID format".to_owned()))?;
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
            let id = key
                .parse::<i32>()
                .map_err(|_| AppError::BadRequest("Invalid numeric location ID".to_owned()))?;
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
        loc_to_media
            .entry(row.location_id)
            .or_default()
            .push(row.media_id);
    }

    let mut resolved = Vec::with_capacity(raw_locs.len());
    for loc in raw_locs {
        let media_item_ids = loc_to_media.get(&loc.id).cloned().unwrap_or_default();
        let thumbnail_id = get_representative_thumbnail(tx, &media_item_ids).await?;

        resolved.push(VisitedLocation {
            id: format!("place:{}", loc.id),
            name: loc.name,
            admin1: Some(loc.admin1),
            admin2: Some(loc.admin2),
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

    let top_locations = sqlx::query_as!(RawLocation,
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

    let mut top_10: Vec<RawLocation> = top_locations.iter().take(10).cloned().collect();
    fastrand::shuffle(&mut top_10);
    let common_locs_raw: Vec<RawLocation> = top_10.into_iter().take(3).collect();

    let mut ranks_11_30: Vec<RawLocation> =
        top_locations.iter().skip(10).take(20).cloned().collect();
    fastrand::shuffle(&mut ranks_11_30);
    let rare_locs_raw: Vec<RawLocation> = ranks_11_30.into_iter().take(3).collect();

    let mut legendary_locs_all = sqlx::query_as!(RawLocation,
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
    fastrand::shuffle(&mut legendary_locs_all);
    let legendary_locs_raw: Vec<RawLocation> = legendary_locs_all.into_iter().take(3).collect();

    // Retrieving Recent Destinations
    let recent_destinations_raw = sqlx::query_as!(
        RawLocation,
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

    let common_locs = resolve_locations(&mut tx, user_id, common_locs_raw).await?;
    let rare_locs = resolve_locations(&mut tx, user_id, rare_locs_raw).await?;
    let legendary_locs = resolve_locations(&mut tx, user_id, legendary_locs_raw).await?;
    let recent_destinations = resolve_locations(&mut tx, user_id, recent_destinations_raw).await?;

    tx.commit().await?;

    let mut all_locations = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for loc in common_locs
        .into_iter()
        .chain(rare_locs)
        .chain(legendary_locs)
        .chain(recent_destinations)
    {
        if seen_ids.insert(loc.id.clone()) {
            all_locations.push(loc);
        }
    }

    fastrand::shuffle(&mut all_locations);
    Ok(all_locations)
}

struct IntermediateLocation {
    name: String,
    admin1: Option<String>,
    admin2: Option<String>,
    country_code: String,
    country_name: String,
    media_item_ids: Vec<String>,
}

async fn get_place_location(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: i32,
    location_id: i32,
) -> Result<IntermediateLocation, AppError> {
    let rows = sqlx::query!(
                r#"
                SELECT m.id as "media_item_id!", l.name, l.admin1, l.admin2, l.country_code, l.country_name
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.id = $2 AND m.deleted = false
                "#,
                user_id,
                location_id
            )
        .fetch_all(executor)
        .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound(format!(
            "Location {location_id} not found"
        )));
    }

    let first = &rows[0];

    Ok(IntermediateLocation {
        name: first.name.clone(),
        admin1: Some(first.admin1.clone()),
        admin2: Some(first.admin2.clone()),
        country_code: first.country_code.clone(),
        country_name: first.country_name.clone(),
        media_item_ids: rows.into_iter().map(|r| r.media_item_id).collect(),
    })
}

async fn get_country_location(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: i32,
    country_code: &str,
) -> Result<IntermediateLocation, AppError> {
    let rows = sqlx::query!(
        r#"
                SELECT m.id as "media_item_id!", l.country_name
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND m.deleted = false
                "#,
        user_id,
        country_code
    )
    .fetch_all(executor)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound(format!(
            "Country {country_code} not found"
        )));
    }

    Ok(IntermediateLocation {
        name: rows[0].country_name.clone(),
        admin1: None,
        admin2: None,
        country_code: country_code.to_owned(),
        country_name: rows[0].country_name.clone(),
        media_item_ids: rows.into_iter().map(|r| r.media_item_id).collect(),
    })
}

async fn get_admin1_location(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: i32,
    country_code: &str,
    admin1: &str,
) -> Result<IntermediateLocation, AppError> {
    let rows = sqlx::query!(
        r#"
                SELECT m.id as "media_item_id!", l.country_name
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND l.admin1 = $3 AND m.deleted = false
                "#,
        user_id,
        country_code,
        admin1
    )
    .fetch_all(executor)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound(format!(
            "Region {admin1} in {country_code} not found"
        )));
    }

    Ok(IntermediateLocation {
        name: rows[0].country_name.clone(),
        admin1: Some(admin1.to_owned()),
        admin2: None,
        country_code: country_code.to_owned(),
        country_name: rows[0].country_name.clone(),
        media_item_ids: rows.into_iter().map(|r| r.media_item_id).collect(),
    })
}

pub async fn get_location(
    pool: &PgPool,
    user_id: i32,
    location_key: &str,
) -> Result<LocationDetailsResponse, AppError> {
    let scope = LocationScope::parse(location_key)?;
    let mut tx = pool.begin().await?;

    let intermedia_location = match &scope {
        LocationScope::Place(id) => get_place_location(&mut *tx, user_id, *id).await?,
        LocationScope::Country(code) => get_country_location(&mut *tx, user_id, code).await?,
        LocationScope::Admin1 {
            country_code,
            admin1,
        } => get_admin1_location(&mut *tx, user_id, country_code, admin1).await?,
    };

    let thumbnail_id =
        get_representative_thumbnail(&mut tx, &intermedia_location.media_item_ids).await?;

    let items = match &scope {
        LocationScope::Place(id) => {
            sqlx::query_as!(
                LocationMediaItem,
                r#"
                SELECT
                    m.id,
                    m.is_video as "is_video!",
                    m.has_thumbnails as "has_thumbnails!",
                    m.duration_ms::INT,
                    (m.width::real / m.height::real) AS "ratio!",
                    g.latitude AS "latitude?",
                    g.longitude AS "longitude?"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                WHERE m.user_id = $1 AND g.location_id = $2 AND m.deleted = false
                ORDER BY m.sort_timestamp, id
                "#,
                user_id,
                *id
            )
            .fetch_all(&mut *tx)
            .await?
        }
        LocationScope::Country(code) => {
            sqlx::query_as!(
                LocationMediaItem,
                r#"
                SELECT
                    m.id,
                    m.is_video as "is_video!",
                    m.has_thumbnails as "has_thumbnails!",
                    m.duration_ms::INT,
                    (m.width::real / m.height::real) AS "ratio!",
                    g.latitude AS "latitude?",
                    g.longitude AS "longitude?"
                FROM media_item m
                JOIN gps g ON m.id = g.media_item_id
                JOIN location l ON g.location_id = l.id
                WHERE m.user_id = $1 AND l.country_code = $2 AND m.deleted = false
                ORDER BY m.sort_timestamp, id
                "#,
                user_id,
                code
            )
            .fetch_all(&mut *tx)
            .await?
        }
        LocationScope::Admin1 {
            country_code,
            admin1,
        } => {
            sqlx::query_as!(
                LocationMediaItem,
                r#"
                SELECT
                    m.id,
                    m.is_video as "is_video!",
                    m.has_thumbnails as "has_thumbnails!",
                    m.duration_ms::INT,
                    (m.width::real / m.height::real) AS "ratio!",
                    g.latitude AS "latitude?",
                    g.longitude AS "longitude?"
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
            .fetch_all(&mut *tx)
            .await?
        }
    };

    tx.commit().await?;

    Ok(LocationDetailsResponse {
        location: Some(VisitedLocation {
            id: location_key.to_string(),
            name: intermedia_location.name,
            admin1: intermedia_location.admin1,
            admin2: intermedia_location.admin2,
            country_code: intermedia_location.country_code,
            country_name: intermedia_location.country_name,
            photo_count: items.len() as i64,
            thumbnail_id,
        }),
        items,
    })
}
