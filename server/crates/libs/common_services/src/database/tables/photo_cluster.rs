use pgvector::Vector;

#[derive(Debug, Clone)]
pub struct ExistingPhotoCluster {
    pub id: String,
    pub friendly_label: Option<String>,
    pub centroid: Option<Vector>,
}
