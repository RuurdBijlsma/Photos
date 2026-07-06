use serde::{Deserialize, Serialize};

/// Corresponds to the '`capture_details`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CameraSettings {
    pub iso: Option<i32>,
    pub exposure_time: Option<f32>,
    pub aperture: Option<f32>,
    pub focal_length: Option<f32>,
    pub focal_length_in_35mm: Option<f32>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub flash_fired: Option<bool>,
    pub flash_mode: Option<String>,
    pub lens_make: Option<String>,
    pub lens_model: Option<String>,
    pub digital_zoom_ratio: Option<f32>,
    pub subject_distance: Option<f32>,
    pub exposure_compensation: Option<f32>,
}

/// Converts from the analysis result's `SourceCaptureDetails` to the database model `CaptureDetails`.
impl From<media_analyzer::CameraSettings> for CameraSettings {
    fn from(details: media_analyzer::CameraSettings) -> Self {
        Self {
            iso: details.iso.map(|iso| iso as i32),
            exposure_time: details.exposure_time.map(|et| et as f32),
            aperture: details.aperture.map(|a| a as f32),
            focal_length: details.focal_length.map(|fl| fl as f32),
            camera_make: details.camera_make,
            camera_model: details.camera_model,
            flash_fired: details.flash.clone().map(|f| f.fired),
            flash_mode: details.flash.map(|f| f.mode.as_str().to_owned()),
            lens_make: details.lens_make,
            lens_model: details.lens_model,
            digital_zoom_ratio: details.digital_zoom_ratio.map(|fl| fl as f32),
            subject_distance: details.subject_distance.map(|fl| fl as f32),
            focal_length_in_35mm: details.focal_length_in_35mm.map(|fl| fl as f32),
            exposure_compensation: details.exposure_compensation.map(|fl| fl as f32),
        }
    }
}
