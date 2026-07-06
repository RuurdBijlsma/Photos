use crate::database::UpdateField;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeoPhotosParams {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadMediaParams {
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PhotoThumbnailParams {
    pub size: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMediaItemRequest {
    #[serde(default)]
    pub user_caption: UpdateField<String>,
    #[serde(default)]
    pub taken_at_local: Option<String>,
    #[serde(default)]
    pub use_panorama_viewer: Option<bool>,
    #[serde(default)]
    pub timezone_offset_seconds: UpdateField<i32>,
}
