use common_types::ml_analysis;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Corresponds to the '`color_data`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorData {
    pub prominent_colors: Vec<String>,
    pub average_hue: f32,
    pub average_saturation: f32,
    pub average_lightness: f32,
    pub histogram: Value,
}

impl From<ml_analysis::ColorData> for ColorData {
    fn from(color_data: ml_analysis::ColorData) -> Self {
        Self {
            prominent_colors: color_data.prominent_colors,
            average_hue: color_data.average_hue,
            average_saturation: color_data.average_saturation,
            average_lightness: color_data.average_lightness,
            // Serialize the nested histogram struct to a JSON Value
            histogram: json!(color_data.histogram),
        }
    }
}
