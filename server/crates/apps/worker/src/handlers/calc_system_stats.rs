use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::Result;
use common_services::database::jobs::Job;
use common_services::database::key_json_store::KeyJsonStore;
use common_services::database::user_store::UserStore;
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};

#[derive(serde::Deserialize, serde::Serialize)]
struct ItemCoord {
    latitude: f64,
    longitude: f64,
}

async fn store_empty_vars(pool: &PgPool) -> Result<()> {
    let users = UserStore::list_user_ids(pool).await?;
    for user_id in users {
        KeyJsonStore::set_value(
            pool,
            "total_area_sq_km",
            &serde_json::json!(0.0),
            Some(user_id),
        )
        .await?;
        KeyJsonStore::set_value(
            pool,
            "most_frequent_location",
            &serde_json::Value::Null,
            Some(user_id),
        )
        .await?;
    }

    Ok(())
}

fn find_most_frequent_location(coordinates: &Vec<ItemCoord>) -> Option<ItemCoord> {
    // Cluster by lat/lon coordinates with HDBSCAN
    let min_cluster_size = 2;
    let min_samples = 2;

    let embeddings: Vec<Vec<f32>> = coordinates
        .iter()
        .map(|c| vec![c.latitude as f32, c.longitude as f32])
        .collect();

    let mut centroid_lat = None;
    let mut centroid_lon = None;

    if embeddings.len() > min_samples {
        match crate::handlers::common::clustering::run_hdbscan(
            &embeddings,
            min_cluster_size,
            min_samples,
        ) {
            Ok((labels, centroids)) => {
                let mut cluster_counts = HashMap::new();
                for &label in &labels {
                    if label >= 0 {
                        *cluster_counts.entry(label).or_insert(0) += 1;
                    }
                }

                let largest_cluster = cluster_counts.into_iter().max_by_key(|&(_, count)| count);

                if let Some((label, _count)) = largest_cluster
                    && let Some(centroid) = centroids.get(label as usize)
                    && centroid.len() >= 2
                {
                    centroid_lat = Some(f64::from(centroid[0]));
                    centroid_lon = Some(f64::from(centroid[1]));
                }
            }
            Err(e) => {
                warn!(
                    "HDBSCAN clustering failed during system stats calculation: {:?}",
                    e
                );
            }
        }
    }

    // Fallback: If HDBSCAN didn't return a valid cluster centroid but data exists,
    // find the most recurring coordinate based on high resolution rounding.
    if centroid_lat.is_none() && !coordinates.is_empty() {
        let mut coord_counts = HashMap::new();
        for coord in coordinates {
            let lat_key = (coord.latitude * 10000.0).round() as i32;
            let lon_key = (coord.longitude * 10000.0).round() as i32;
            *coord_counts.entry((lat_key, lon_key)).or_insert(0) += 1;
        }

        if let Some(((lat_key, lon_key), _)) =
            coord_counts.into_iter().max_by_key(|&(_, count)| count)
        {
            centroid_lat = Some(f64::from(lat_key) / 10000.0);
            centroid_lon = Some(f64::from(lon_key) / 10000.0);
        }
    }

    centroid_lat
        .zip(centroid_lon)
        .map(|(latitude, longitude)| ItemCoord {
            latitude,
            longitude,
        })
}

#[allow(clippy::suspicious_operation_groupings)]
fn calculate_coordinates_area(coordinates: &Vec<ItemCoord>) -> f64 {
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;
    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;

    for coord in coordinates {
        if coord.latitude < min_lat {
            min_lat = coord.latitude;
        }
        if coord.latitude > max_lat {
            max_lat = coord.latitude;
        }
        if coord.longitude < min_lon {
            min_lon = coord.longitude;
        }
        if coord.longitude > max_lon {
            max_lon = coord.longitude;
        }
    }

    // Earth's mean radius in kilometers (WGS-84)
    let r = 6371.0088_f64;
    let lat1_rad = min_lat.to_radians();
    let lat2_rad = max_lat.to_radians();
    let lon1_rad = min_lon.to_radians();
    let lon2_rad = max_lon.to_radians();

    // Span in longitude (adjust for potential antimeridian crossing bounds)
    let delta_lon = if lon2_rad >= lon1_rad {
        lon2_rad - lon1_rad
    } else {
        2.0f64.mul_add(std::f64::consts::PI, lon2_rad - lon1_rad)
    };

    // Calculate spherical area of the bounding box sector: R^2 * (sin(lat2) - sin(lat1)) * delta_lon
    r * r * (lat2_rad.sin() - lat1_rad.sin()) * delta_lon
}

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    info!("📊 calc system stats...");

    let users = UserStore::list_user_ids(&context.pool).await?;
    for user_id in users {
        // Query all latitudes and longitudes from non-deleted media items
        let coordinates = sqlx::query_as!(
            ItemCoord,
            r#"
                SELECT gps.latitude, gps.longitude
                FROM gps
                JOIN media_item ON media_item.id = gps.media_item_id
                WHERE media_item.deleted = false AND user_id = $1
            "#,
            user_id
        )
        .fetch_all(&context.pool)
        .await?;

        if coordinates.is_empty() {
            info!("No geotagged media items found. Storing default empty stats.");
            store_empty_vars(&context.pool).await?;
            return Ok(JobResult::Done);
        }

        // Calc & store area
        let total_area_val = serde_json::json!(calculate_coordinates_area(&coordinates));
        KeyJsonStore::set_value(
            &context.pool,
            "total_area_sq_km",
            &total_area_val,
            Some(user_id),
        )
        .await?;

        // Calc and store most frequent location
        let location_val =
            find_most_frequent_location(&coordinates).map_or(serde_json::Value::Null, |centroid| {
                serde_json::json!({
                    "latitude": centroid.latitude,
                    "longitude": centroid.longitude,
                })
            });
        KeyJsonStore::set_value(
            &context.pool,
            "most_frequent_location",
            &location_val,
            Some(user_id),
        )
        .await?;

        info!(
            "Saved system stats separately - total_area_sq_km: {}, most_frequent_location: {:?}",
            total_area_val, location_val
        );
    }

    Ok(JobResult::Done)
}
