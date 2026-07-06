use pgvector::Vector;

#[derive(Debug, Clone)]
pub struct ExistingFaceCluster {
    pub id: String,
    pub person_id: String,
    pub centroid: Option<Vector>,
}
