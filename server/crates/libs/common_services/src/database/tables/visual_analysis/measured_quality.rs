use common_types::ml_analysis::QualityMeasurement;
use serde::{Deserialize, Serialize};

/// Corresponds to the '`measured_quality`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeasuredQuality {
    pub blurriness: f64,
    pub noisiness: f64,
    pub exposure: f64,
    pub weighted_score: f64,
    pub accidentalness: f64,
}

impl From<QualityMeasurement> for MeasuredQuality {
    fn from(measured: QualityMeasurement) -> Self {
        Self {
            noisiness: measured.noisiness,
            exposure: measured.exposure,
            blurriness: measured.blurriness,
            weighted_score: measured.weighted_score,
            accidentalness: measured.accidentalness,
        }
    }
}
