use color_eyre::{Result, eyre::eyre};
use common_services::database::jobs::Job;
use common_services::database::user_store::UserStore;
use hdbscan::{Center, DistanceMetric, Hdbscan, HdbscanHyperParams};
use pgvector::Vector;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tracing::debug;

pub trait ClusterEntity {
    fn id(&self) -> String;
    fn centroid(&self) -> Option<&Vector>;
}

/// Normalizes a vector in-place to unit length (L2 norm = 1.0).
pub fn normalize_vector(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Calculates the L2 (Euclidean) distance between two equal-length vectors.
pub fn l2_distance(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(eyre!("Vectors must have the same dimension"));
    }
    Ok(a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt())
}

/// Runs the HDBSCAN algorithm to find clusters and their centroids.
pub fn run_hdbscan(
    embeddings: &[Vec<f32>],
    min_cluster_size: usize,
    min_samples: usize,
) -> Result<(Vec<i32>, Vec<Vec<f32>>)> {
    if embeddings.len() <= min_samples {
        return Ok((Vec::new(), Vec::new()));
    }
    let params = HdbscanHyperParams::builder()
        .min_cluster_size(min_cluster_size)
        .min_samples(min_samples)
        .allow_single_cluster(false)
        .dist_metric(DistanceMetric::Euclidean)
        .build();

    let clusterer = Hdbscan::new(embeddings, params);
    let now = Instant::now();
    let labels = clusterer.cluster()?;
    debug!("HDBSCAN clustering finished in {:?}", now.elapsed());

    let mut centroids = clusterer.calc_centers(Center::Centroid, &labels)?;
    // Normalize centroids back to unit length so distances remain consistent
    for centroid in &mut centroids {
        normalize_vector(centroid);
    }

    Ok((labels, centroids))
}

/// Matches new centroids to existing clusters using globally sorted best-distance pairs.
pub fn match_centroids<T: ClusterEntity>(
    new_centroids: &[Vec<f32>],
    existing_clusters: &[T],
    threshold: f32,
) -> Result<HashMap<usize, String>> {
    let mut candidates = Vec::new();

    for (new_cid, new_centroid) in new_centroids.iter().enumerate() {
        for existing_cluster in existing_clusters {
            if let Some(existing_centroid) = existing_cluster.centroid() {
                let distance = l2_distance(new_centroid.as_slice(), existing_centroid.as_slice())?;
                if distance < threshold {
                    candidates.push((distance, new_cid, existing_cluster.id()));
                }
            }
        }
    }

    // Sort by smallest distance first so the best global pairings win
    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut map = HashMap::new();
    let mut matched_new = HashSet::new();
    let mut used_old = HashSet::new();

    for (_dist, new_cid, old_id) in candidates {
        if !matched_new.contains(&new_cid) && !used_old.contains(&old_id) {
            matched_new.insert(new_cid);
            used_old.insert(old_id.clone());
            map.insert(new_cid, old_id);
        }
    }

    Ok(map)
}

/// Groups items into a map based on their assigned cluster label.
pub fn group_by_cluster<'a, T>(labels: &[i32], items: &'a [T]) -> HashMap<usize, Vec<&'a T>> {
    let mut clusters: HashMap<usize, Vec<&'a T>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        if label >= 0 {
            clusters.entry(label as usize).or_default().push(&items[i]);
        }
    }
    clusters
}

pub async fn fetch_user_ids(pool: &PgPool, job: &Job) -> Result<Vec<i32>> {
    if let Some(user_id) = job.user_id {
        Ok(vec![user_id])
    } else {
        Ok(UserStore::list_user_ids(pool).await?)
    }
}

/// Calculates the Great Circle distance (in meters) between two GPS coordinates using the Haversine formula.
#[allow(clippy::suboptimal_flops)]
#[must_use]
pub fn haversine_distance_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).max(0.0).sqrt());

    EARTH_RADIUS_METERS * c
}

/// A media item with temporal, spatial, and semantic embedding attributes for session clustering.
#[derive(Debug, Clone)]
pub struct MediaItemForClustering {
    pub media_item_id: String,
    pub sort_timestamp: chrono::DateTime<chrono::Utc>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub embedding: Vec<f32>,
}

/// A session of media items taken close in time and space, with an L2-normalized centroid embedding.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: usize,
    pub items: Vec<MediaItemForClustering>,
    pub centroid: Vec<f32>,
}

/// Computes the L2-normalized centroid (mean embedding) for a slice of items.
fn compute_centroid(items: &[MediaItemForClustering]) -> Vec<f32> {
    if items.is_empty() {
        return Vec::new();
    }
    if items.len() == 1 {
        let mut centroid = items[0].embedding.clone();
        normalize_vector(&mut centroid);
        return centroid;
    }

    let dim = items[0].embedding.len();
    let mut centroid = vec![0.0f32; dim];
    for item in items {
        for (i, &val) in item.embedding.iter().enumerate() {
            if i < dim {
                centroid[i] += val;
            }
        }
    }
    normalize_vector(&mut centroid);
    centroid
}

/// Partitions media items (assumed sorted by `sort_timestamp ASC`) into distinct sessions based on time and distance thresholds.
/// Calculates the L2-normalized centroid embedding for each session.
#[must_use]
pub fn segment_into_sessions(
    items: &[MediaItemForClustering],
    session_time_gap_seconds: u64,
    session_distance_meters: f64,
) -> Vec<Session> {
    if items.is_empty() {
        return Vec::new();
    }

    let mut sessions: Vec<Vec<MediaItemForClustering>> = Vec::new();
    let mut current_session: Vec<MediaItemForClustering> = Vec::new();

    for item in items {
        if let Some(prev) = current_session.last() {
            let time_gap = (item.sort_timestamp - prev.sort_timestamp)
                .num_seconds()
                .unsigned_abs();
            let distance_exceeded =
                match (prev.latitude, prev.longitude, item.latitude, item.longitude) {
                    (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) => {
                        haversine_distance_meters(lat1, lon1, lat2, lon2) > session_distance_meters
                    }
                    _ => false,
                };
            let split = time_gap > session_time_gap_seconds || distance_exceeded;

            if split {
                sessions.push(std::mem::take(&mut current_session));
            }
        }
        current_session.push(item.clone());
    }

    if !current_session.is_empty() {
        sessions.push(current_session);
    }

    sessions
        .into_iter()
        .enumerate()
        .map(|(session_id, session_items)| {
            let centroid = compute_centroid(&session_items);
            Session {
                session_id,
                items: session_items,
                centroid,
            }
        })
        .collect()
}

/// Computes cosine similarity between two unit-normalized vectors (dot product).
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Candidate photo item for diversity sampling via MMR.
#[derive(Debug, Clone)]
pub struct CandidatePhoto {
    pub media_item_id: String,
    pub session_id: i32,
    pub embedding: Vec<f32>,
}

/// Selects diverse and theme-relevant photos using Maximal Marginal Relevance (MMR).
/// Guarantees at most 1 photo is picked per `session_id`.
#[allow(clippy::suboptimal_flops)]
#[must_use]
pub fn select_mmr_photos(
    candidates: &[CandidatePhoto],
    theme_centroid: &[f32],
    max_photos: usize,
    relevance_lambda: f32,
) -> Vec<usize> {
    if candidates.is_empty() || max_photos == 0 {
        return Vec::new();
    }

    let mut selected_indices: Vec<usize> = Vec::with_capacity(max_photos);
    let mut used_sessions: HashSet<i32> = HashSet::new();

    // Pre-calculate relevance of all candidates to the theme centroid
    let relevance: Vec<f32> = candidates
        .iter()
        .map(|c| cosine_similarity(&c.embedding, theme_centroid))
        .collect();

    // Step 1: Pick the candidate most relevant to the theme centroid
    let first_pick = candidates
        .iter()
        .enumerate()
        .max_by(|(idx_a, _), (idx_b, _)| {
            relevance[*idx_a]
                .partial_cmp(&relevance[*idx_b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some((first_idx, first_cand)) = first_pick {
        selected_indices.push(first_idx);
        used_sessions.insert(first_cand.session_id);
    } else {
        return selected_indices;
    }

    // Step 2: Iteratively pick remaining photos with MMR, skipping already used sessions
    while selected_indices.len() < max_photos {
        let mut best_score = f32::NEG_INFINITY;
        let mut best_candidate_idx: Option<usize> = None;

        for (cand_idx, cand) in candidates.iter().enumerate() {
            if used_sessions.contains(&cand.session_id) {
                continue;
            }

            // Redundancy = max similarity to any already selected photo
            let max_similarity_to_selected = selected_indices
                .iter()
                .map(|&sel_idx| cosine_similarity(&cand.embedding, &candidates[sel_idx].embedding))
                .fold(f32::NEG_INFINITY, f32::max);

            let score = relevance_lambda * relevance[cand_idx]
                - (1.0 - relevance_lambda) * max_similarity_to_selected;

            if score > best_score {
                best_score = score;
                best_candidate_idx = Some(cand_idx);
            }
        }

        if let Some(next_idx) = best_candidate_idx {
            selected_indices.push(next_idx);
            used_sessions.insert(candidates[next_idx].session_id);
        } else {
            // No more candidates from unused sessions
            break;
        }
    }

    selected_indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_haversine_distance() {
        // Paris (48.8566, 2.3522) to London (51.5074, -0.1278) is approx ~343 km
        let dist = haversine_distance_meters(48.8566, 2.3522, 51.5074, -0.1278);
        assert!((dist - 343_500.0).abs() < 5000.0);

        // Same point should be 0
        let zero_dist = haversine_distance_meters(52.0, 4.0, 52.0, 4.0);
        assert!(zero_dist < 0.001);
    }

    #[test]
    fn test_segment_into_sessions_empty() {
        let items: Vec<MediaItemForClustering> = Vec::new();
        let sessions = segment_into_sessions(&items, 1800, 300.0);
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_segment_into_sessions_time_and_gps() {
        let base_time = Utc::now();
        let items = vec![
            // Session 0: Item 1 & 2 close in time (5 min) and location (0m)
            MediaItemForClustering {
                media_item_id: "photo1".to_string(),
                sort_timestamp: base_time,
                latitude: Some(52.3702),
                longitude: Some(4.8952),
                embedding: vec![1.0, 0.0],
            },
            MediaItemForClustering {
                media_item_id: "photo2".to_string(),
                sort_timestamp: base_time + Duration::minutes(5),
                latitude: Some(52.3702),
                longitude: Some(4.8952),
                embedding: vec![0.0, 1.0],
            },
            // Session 1: Item 3 is 45 mins later (gap > 30 min)
            MediaItemForClustering {
                media_item_id: "photo3".to_string(),
                sort_timestamp: base_time + Duration::minutes(50),
                latitude: Some(52.3702),
                longitude: Some(4.8952),
                embedding: vec![1.0, 1.0],
            },
            // Session 2: Item 4 is 5 mins later, but 10 km away
            MediaItemForClustering {
                media_item_id: "photo4".to_string(),
                sort_timestamp: base_time + Duration::minutes(55),
                latitude: Some(52.4500),
                longitude: Some(5.0000),
                embedding: vec![0.5, 0.5],
            },
        ];

        let sessions = segment_into_sessions(&items, 1800, 300.0);
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].items.len(), 2);
        assert_eq!(sessions[1].items.len(), 1);
        assert_eq!(sessions[2].items.len(), 1);
        assert_eq!(sessions[0].session_id, 0);
        assert_eq!(sessions[1].session_id, 1);
        assert_eq!(sessions[2].session_id, 2);
    }

    #[test]
    fn test_segment_into_sessions_missing_gps() {
        let base_time = Utc::now();
        let items = vec![
            MediaItemForClustering {
                media_item_id: "p1".to_string(),
                sort_timestamp: base_time,
                latitude: None,
                longitude: None,
                embedding: vec![1.0, 0.0],
            },
            MediaItemForClustering {
                media_item_id: "p2".to_string(),
                sort_timestamp: base_time + Duration::minutes(10),
                latitude: None,
                longitude: None,
                embedding: vec![0.0, 1.0],
            },
            MediaItemForClustering {
                media_item_id: "p3".to_string(),
                sort_timestamp: base_time + Duration::minutes(60),
                latitude: None,
                longitude: None,
                embedding: vec![1.0, 1.0],
            },
        ];

        let sessions = segment_into_sessions(&items, 1800, 300.0);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].items.len(), 2);
        assert_eq!(sessions[1].items.len(), 1);
    }

    #[test]
    fn test_select_mmr_photos_max_one_per_session() {
        let theme_centroid = vec![1.0, 0.0];
        let candidates = vec![
            CandidatePhoto {
                media_item_id: "photo_0".to_string(),
                session_id: 10,
                embedding: vec![1.0, 0.0],
            },
            CandidatePhoto {
                media_item_id: "photo_1_duplicate_session".to_string(),
                session_id: 10,
                embedding: vec![0.95, 0.05],
            },
            CandidatePhoto {
                media_item_id: "photo_2".to_string(),
                session_id: 20,
                embedding: vec![0.8, 0.2],
            },
            CandidatePhoto {
                media_item_id: "photo_3".to_string(),
                session_id: 30,
                embedding: vec![0.7, 0.3],
            },
        ];

        let picks = select_mmr_photos(&candidates, &theme_centroid, 3, 0.6);
        assert_eq!(picks.len(), 3);
        assert_eq!(picks[0], 0); // Closest to centroid
        // Ensure photo_1 (which has session 10) was NOT picked
        assert!(!picks.contains(&1));
        assert!(picks.contains(&2));
        assert!(picks.contains(&3));
    }
}
