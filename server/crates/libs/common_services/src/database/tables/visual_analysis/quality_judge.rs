use common_types::ml_analysis::MLLlmQualityJudgement;
use serde::{Deserialize, Serialize};

/// Corresponds to the '`judged_quality`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityJudge {
    pub exposure: u8,
    pub contrast: u8,
    pub sharpness: u8,
    pub color_accuracy: u8,
    pub composition: u8,
    pub subject_clarity: u8,
    pub visual_impact: u8,
    pub creativity: u8,
    pub color_harmony: u8,
    pub storytelling: u8,
    pub style_suitability: u8,
    pub weighted_score: f64,
}

impl From<MLLlmQualityJudgement> for QualityJudge {
    fn from(judged: MLLlmQualityJudgement) -> Self {
        Self {
            exposure: judged.exposure,
            contrast: judged.contrast,
            sharpness: judged.sharpness,
            color_accuracy: judged.color_accuracy,
            composition: judged.composition,
            subject_clarity: judged.subject_clarity,
            visual_impact: judged.visual_impact,
            creativity: judged.creativity,
            color_harmony: judged.color_harmony,
            storytelling: judged.storytelling,
            style_suitability: judged.style_suitability,
            weighted_score: f64::from(judged.weighted_score()),
        }
    }
}
