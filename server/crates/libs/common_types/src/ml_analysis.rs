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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct MLFastAnalysis {
    pub percentage: i32,
    pub color_data: ColorData,
    pub embedding: Vec<f32>,
    pub faces: Vec<FaceAnalysis>,
    pub objects: Vec<DetectedObject>,
    pub measured_quality: QualityMeasurement,
}