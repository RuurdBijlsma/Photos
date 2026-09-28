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
