use color_eyre::{Result, eyre::eyre};
use common_services::database::jobs::Job;
use common_services::database::user_store::UserStore;
use hdbscan::{Center, DistanceMetric, Hdbscan, HdbscanHyperParams};
use pgvector::Vector;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub trait ClusterEntity {
    fn id(&self) -> String;
    fn centroid(&self) -> Option<&Vector>;
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
    println!("clusterer.cluster()?, {:?}", now.elapsed());
    let centroids = clusterer.calc_centers(Center::Centroid, &labels)?;
    Ok((labels, centroids))
}

/// Matches new centroids to existing clusters if they are within a distance threshold.
pub fn match_centroids<T: ClusterEntity>(
    new_centroids: &[Vec<f32>],
    existing_clusters: &[T],
    threshold: f32,
) -> Result<HashMap<usize, String>> {
    let mut map = HashMap::new();
    let mut used_old_ids = HashSet::new();

    for (new_cid, new_centroid) in new_centroids.iter().enumerate() {
        let mut best_match: Option<(String, f32)> = None;
        for existing_cluster in existing_clusters {
            if let Some(existing_centroid) = existing_cluster.centroid() {
                let distance = l2_distance(new_centroid.as_slice(), existing_centroid.as_slice())?;
                if distance < threshold {
                    if let Some((_, best_dist)) = best_match {
                        if distance < best_dist {
                            best_match = Some((existing_cluster.id(), distance));
                        }
                    } else {
                        best_match = Some((existing_cluster.id(), distance));
                    }
                }
            }
        }

        if let Some((id, _)) = best_match
            && used_old_ids.insert(id.clone())
        {
            map.insert(new_cid, id.clone());
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
