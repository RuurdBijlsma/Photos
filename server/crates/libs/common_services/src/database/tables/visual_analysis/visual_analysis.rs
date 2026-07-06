use crate::database::visual_analysis::caption_data::ClassificationData;
use crate::database::visual_analysis::color_data::ColorData;
use crate::database::visual_analysis::face::{CreateFace, Face};
use crate::database::visual_analysis::measured_quality::MeasuredQuality;
use crate::database::visual_analysis::object::Object;
use crate::database::visual_analysis::quality_judge::QualityJudge;
use chrono::{DateTime, Utc};
use common_types::ml_analysis::MLFastAnalysis;
use pgvector::Vector;
use serde::{Deserialize, Serialize};

/// Represents a single photo's embedding data fetched for clustering.
#[derive(Debug, Clone)]
pub struct MediaEmbedding {
    pub media_item_id: String,
    pub embedding: Vector,
}

/// A composite struct representing a '`visual_analysis`' run and all its associated nested data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateVisualAnalysis {
    pub embedding: Vector,
    pub percentage: i32,
    pub faces: Vec<CreateFace>,
    pub colors: ColorData,
    pub objects: Vec<Object>,
    pub quality: MeasuredQuality,
}

impl From<MLFastAnalysis> for CreateVisualAnalysis {
    fn from(data: MLFastAnalysis) -> Self {
        Self {
            embedding: data.embedding.into(),
            percentage: data.percentage,
            faces: data.faces.into_iter().map(Into::into).collect(),
            colors: data.color_data.into(),
            objects: data.objects.into_iter().map(Into::into).collect(),
            quality: data.measured_quality.into(),
        }
    }
}

/// A composite struct representing a '`visual_analysis`' run and all its associated nested data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadVisualAnalysis {
    pub created_at: DateTime<Utc>,
    pub percentage: i32,
    pub faces: Vec<Face>,
    pub objects: Vec<Object>,
    pub colors: ColorData,
    pub measured_quality: MeasuredQuality,
    pub quality_judge: Option<QualityJudge>,
    pub classification: Option<ClassificationData>,
}
