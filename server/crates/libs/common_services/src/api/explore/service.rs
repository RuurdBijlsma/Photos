use crate::api::app_error::AppError;
use crate::api::explore::interfaces::{ExploreTableQuery, PaginatedExploreTableResponse, ExploreMediaItem};
use sqlx::{PgPool, Postgres, QueryBuilder};

pub struct ExploreSort {
    pub column: &'static str,
    pub direction: &'static str,
}

/// Maps safe API fields to qualified database columns with table aliases.
/// Prevents SQL injection and avoids ambiguous column database errors on joins.
#[must_use]
pub fn map_explore_field_to_column(field: &str) -> Option<&'static str> {
    match field {
        "id" => Some("m.id"),
        "filename" => Some("m.filename"),
        "is_video" | "isVideo" => Some("m.is_video"),
        "taken_at_local" | "takenAtLocal" => Some("m.taken_at_local"),
        "duration_ms" | "durationMs" => Some("m.duration_ms"),
        "latitude" => Some("g.latitude"),
        "longitude" => Some("g.longitude"),
        "altitude" => Some("g.altitude"),
        "temp" | "temperature" => Some("w.temperature"),
        "wind_speed" | "windSpeed" => Some("w.wind_speed"),
        "iso" => Some("c.iso"),
        "exposure_time" | "exposureTime" | "shutter_speed" | "shutterSpeed" => Some("c.exposure_time"),
        "aperture" => Some("c.aperture"),
        "focal_length" | "focalLength" => Some("c.focal_length"),
        "size_bytes" | "sizeBytes" => Some("f.size_bytes"),
        _ => None,
    }
}

pub fn parse_explore_sort(sort_str: &str) -> Result<ExploreSort, AppError> {
    let parts: Vec<&str> = sort_str.splitn(2, ':').collect();
    if parts.is_empty() {
        return Err(AppError::BadRequest("Sort parameter cannot be empty".to_owned()));
    }

    let raw_field = parts[0];
    let column = map_explore_field_to_column(raw_field)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid sort field: {raw_field}")))?;

    let direction = if parts.len() == 2 {
        let dir = parts[1].to_lowercase();
        if dir == "desc" || dir == "descending" {
            "DESC"
        } else if dir == "asc" || dir == "ascending" {
            "ASC"
        } else {
            return Err(AppError::BadRequest(format!(
                "Invalid sort direction '{dir}'. Must be 'asc' or 'desc'"
            )));
        }
    } else {
        "ASC"
    };

    Ok(ExploreSort { column, direction })
}

pub async fn get_explore_table(
    pool: &PgPool,
    user_id: i32,
    query: ExploreTableQuery,
) -> Result<PaginatedExploreTableResponse, AppError> {
    // 1. Parse and validate sorting parameters
    let mut sorts = Vec::new();
    for s_str in &query.sort {
        for part in s_str.split(',') {
            let part_trimmed = part.trim();
            if !part_trimmed.is_empty() {
                sorts.push(parse_explore_sort(part_trimmed)?);
            }
        }
    }

    // Default sorting order if none was provided
    if sorts.is_empty() {
        sorts.push(ExploreSort {
            column: "m.taken_at_local",
            direction: "DESC",
        });
        sorts.push(ExploreSort {
            column: "m.id",
            direction: "DESC",
        });
    }

    // 2. Pagination limits
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = if let Some(off) = query.offset {
        off.max(0)
    } else if let Some(pg) = query.page {
        ((pg - 1).max(0)) * limit
    } else {
        0
    };

    // 3. Count matching active media items (highly selective, no joins needed for count)
    let mut count_builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM media_item m");
    count_builder.push(" WHERE m.user_id = ");
    count_builder.push_bind(user_id);
    count_builder.push(" AND m.deleted = false");

    let count_query = count_builder.build_query_scalar::<i64>();
    let total = count_query.fetch_one(pool).await?;

    // 4. Build paginated select query with joins
    let mut select_builder = QueryBuilder::<Postgres>::new(
        "SELECT \
            m.id, \
            m.filename, \
            m.is_video, \
            m.has_thumbnails, \
            m.duration_ms, \
            m.taken_at_local, \
            g.latitude, \
            g.longitude, \
            g.altitude, \
            w.temperature, \
            w.wind_speed, \
            c.iso, \
            c.exposure_time, \
            c.aperture, \
            c.focal_length, \
            f.size_bytes \
         FROM media_item m \
         LEFT JOIN gps g ON m.id = g.media_item_id \
         LEFT JOIN weather w ON m.id = w.media_item_id \
         LEFT JOIN camera_settings c ON m.id = c.media_item_id \
         LEFT JOIN media_features f ON m.id = f.media_item_id \
         WHERE m.user_id = "
    );
    select_builder.push_bind(user_id);
    select_builder.push(" AND m.deleted = false");

    // Dynamic sorting
    select_builder.push(" ORDER BY ");
    for (i, sort) in sorts.iter().enumerate() {
        if i > 0 {
            select_builder.push(", ");
        }
        select_builder.push(sort.column);
        if sort.direction == "DESC" {
            select_builder.push(" DESC NULLS LAST");
        } else {
            select_builder.push(" ASC NULLS LAST");
        }
    }

    // Dynamic pagination
    select_builder.push(" LIMIT ");
    select_builder.push_bind(limit);
    select_builder.push(" OFFSET ");
    select_builder.push_bind(offset);

    let select_query = select_builder.build_query_as::<ExploreMediaItem>();
    let data = select_query.fetch_all(pool).await?;

    Ok(PaginatedExploreTableResponse {
        data,
        total,
        limit,
        offset,
    })
}