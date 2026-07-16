use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExploreTableQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    /// Sorting params, e.g. `sort=temperature:desc&sort=iso:asc`
    #[serde(default)]
    pub sort: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExploreMediaItem {
    pub id: String,
    pub filename: String,
    pub is_video: bool,
    pub has_thumbnails: bool,
    pub duration_ms: Option<i64>,
    pub taken_at_local: NaiveDateTime,

    // GPS fields
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,

    // Weather fields
    pub temperature: Option<f32>,
    pub wind_speed: Option<f32>,

    // Camera settings
    pub iso: Option<i32>,
    pub exposure_time: Option<f32>,
    pub aperture: Option<f32>,
    pub focal_length: Option<f32>,

    // Media features
    pub size_bytes: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedExploreTableResponse {
    pub data: Vec<ExploreMediaItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}