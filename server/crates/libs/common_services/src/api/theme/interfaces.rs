use material_color_utils::MaterializedTheme;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RandomPhotoResponse {
    pub media_id: String,
    pub theme: MaterializedTheme,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ColorThemeParams {
    pub color: String,
    pub variant: String,
    pub contrast: f64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RandomPhotoParams {
    pub variant: String,
    pub contrast: f64,
}
