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
    #[serde(default)]
    pub orientation: Option<i32>,
    #[serde(default)]
    pub gps: UpdateField<GpsEdit>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GpsEdit {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMediaItemResponse {
    pub media_item_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_gps(payload: serde_json::Value) -> UpdateField<GpsEdit> {
        serde_json::from_value::<UpdateMediaItemRequest>(payload)
            .expect("request should parse")
            .gps
    }

    #[test]
    fn gps_is_absent_null_or_value() {
        assert!(parse_gps(serde_json::json!({})).is_ignore());
        assert!(matches!(
            parse_gps(serde_json::json!({ "gps": null })),
            UpdateField::SetNull
        ));
        assert!(matches!(
            parse_gps(serde_json::json!({ "gps": { "latitude": 1.0, "longitude": 2.0 } })),
            UpdateField::Value(_)
        ));
    }
}
