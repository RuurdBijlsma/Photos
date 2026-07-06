use face_id::analyzer::FaceAnalysis;
use object_detector::DetectedObject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ColorData {
    pub prominent_colors: Vec<String>,
    pub average_hue: f32,
    pub average_saturation: f32,
    pub average_lightness: f32,
    pub histogram: ColorHistogram,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ColorHistogram {
    pub bins: i32,
    pub channels: RGBChannels,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RGBChannels {
    pub red: Vec<i32>,
    pub green: Vec<i32>,
    pub blue: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct QualityMeasurement {
    pub blurriness: f64,
    pub noisiness: f64,
    pub exposure: f64,
    pub weighted_score: f64,
    pub accidentalness: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct MLLlmQualityJudgement {
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
}

impl MLLlmQualityJudgement {
    #[must_use]
    pub fn weighted_score(&self) -> f32 {
        let weights = [
            ("exposure", 0.10),
            ("contrast", 0.08),
            ("sharpness", 0.10),
            ("color_accuracy", 0.08),
            ("composition", 0.12),
            ("subject_clarity", 0.12),
            ("visual_impact", 0.10),
            ("creativity", 0.08),
            ("color_harmony", 0.06),
            ("storytelling", 0.12),
            ("style_suitability", 0.04),
        ];

        let values = [
            f32::from(self.exposure),
            f32::from(self.contrast),
            f32::from(self.sharpness),
            f32::from(self.color_accuracy),
            f32::from(self.composition),
            f32::from(self.subject_clarity),
            f32::from(self.visual_impact),
            f32::from(self.creativity),
            f32::from(self.color_harmony),
            f32::from(self.storytelling),
            f32::from(self.style_suitability),
        ];

        values
            .iter()
            .zip(weights.iter())
            .map(|(v, (_, w))| v * w)
            .sum()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct MLLlmClassification {
    pub caption: String,
    pub main_subject: String,
    pub setting: String,
    pub search_term: String,
    pub contains_pets: bool,
    pub contains_vehicle: bool,
    pub contains_landmarks: bool,
    pub contains_people: bool,
    pub contains_animals: bool,
    pub contains_text: bool,
    pub is_indoor: bool,
    pub is_food: bool,
    pub is_drink: bool,
    pub is_event: bool,
    pub is_document: bool,
    pub is_landscape: bool,
    pub is_cityscape: bool,
    pub is_activity: bool,
    pub ocr_text: Option<String>,
    pub animal_type: Option<String>,
    pub food_name: Option<String>,
    pub drink_name: Option<String>,
    pub vehicle_type: Option<String>,
    pub event_type: Option<String>,
    pub landmark_name: Option<String>,
    pub document_type: Option<String>,
    pub people_count: Option<i32>,
    pub people_mood: Option<String>,
    pub photo_type: Option<String>,
    pub activity_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct MLFastAnalysis {
    pub percentage: i32,
    pub color_data: ColorData,
    pub embedding: Vec<f32>,
    pub faces: Vec<FaceAnalysis>,
    pub objects: Vec<DetectedObject>,
    pub measured_quality: QualityMeasurement,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct MLChatAnalysis {
    pub percentage: i32,
    pub quality_judge: Option<MLLlmQualityJudgement>,
    pub llm_classification: MLLlmClassification,
}
