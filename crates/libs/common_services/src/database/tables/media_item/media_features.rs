use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Corresponds to the '`media_features`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateMediaFeatures {
    pub mime_type: String,
    pub size_bytes: i64,
    pub is_motion_photo: bool,
    pub motion_photo_presentation_timestamp: Option<i64>,
    pub is_hdr: bool,
    pub is_burst: bool,
    pub burst_id: Option<String>,
    pub capture_fps: Option<f32>,
    pub video_fps: Option<f32>,
    pub is_nightsight: bool,
    pub is_timelapse: bool,
    pub exif: Value,
    pub compressor_id: Option<String>,
    pub audio_format: Option<String>,
    pub audio_channels: Option<u64>,
    pub audio_sample_rate: Option<u64>,
}

/// Corresponds to the '`media_features`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadMediaFeatures {
    pub mime_type: String,
    pub size_bytes: i64,
    pub is_motion_photo: bool,
    pub motion_photo_presentation_timestamp: Option<i64>,
    pub is_hdr: bool,
    pub is_burst: bool,
    pub burst_id: Option<String>,
    pub capture_fps: Option<f32>,
    pub video_fps: Option<f32>,
    pub is_nightsight: bool,
    pub is_timelapse: bool,
    pub compressor_id: Option<String>,
    pub audio_format: Option<String>,
    pub audio_channels: Option<u64>,
    pub audio_sample_rate: Option<u64>,
}
