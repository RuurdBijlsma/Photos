use chrono::{DateTime, NaiveDate, Utc};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilterRanges {
    pub available_months: Vec<NaiveDate>,
    pub people: Vec<(String, String)>,
    pub countries: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct SearchMediaConfig {
    pub embedder_model_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub semantic_score_threshold: f64,
    pub semantic_weight: f64,
    pub text_weight: f64,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub media_type: SearchMediaType,
    pub sort_by: SearchSortBy,
    pub negative_query: Option<String>,
    pub country_codes: Vec<String>,
    pub person_ids: Vec<String>,
    pub all_faces_required: bool,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    pub query: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub media_type: SearchMediaType,
    #[serde(default)]
    pub sort_by: SearchSortBy,
    pub negative_query: Option<String>,
    pub country_codes: Option<String>,
    pub person_ids: Option<String>,
    pub all_faces_required: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SearchSuggestionsParams {
    pub query: String,
    pub limit: Option<i64>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMediaType {
    #[default]
    All,
    Photo,
    Video,
    Panorama,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchSortBy {
    #[default]
    Relevancy,
    Date,
}

#[derive(Debug, Clone)]
pub struct SearchImage {
    pub session_id: Uuid,
    pub image: Option<DynamicImage>,
}

pub enum VisionQuery {
    Raw(SearchImage),
    Embedding(Vec<f32>),
}
