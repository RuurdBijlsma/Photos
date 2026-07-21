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
    pub relative_humidity: Option<i32>,
    pub precipitation: Option<f32>,
    pub snow: Option<i32>,

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

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistogramResponse {
    pub day_of_week: Vec<DayOfWeekBucket>,
    pub week_of_year: Vec<WeekOfYearBucket>,
    pub hour_of_day: Vec<HourOfDayBucket>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DayOfWeekBucket {
    pub day: i32,
    pub label: String,
    pub count: i64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WeekOfYearBucket {
    pub week: i32,
    pub count: i64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HourOfDayBucket {
    pub hour: i32,
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VisitedLocation {
    pub id: String,
    pub name: String,
    pub admin1: String,
    pub admin2: String,
    pub country_code: String,
    pub country_name: String,
    pub photo_count: i64,
    pub thumbnail_id: Option<String>,
}