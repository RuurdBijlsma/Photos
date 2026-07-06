use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCardResponse {
    pub id: i32,
    pub card_date: Option<NaiveDate>,
    pub card_type: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub thumbnail_media_item_id: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyCardsQueryParams {
    pub date: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateMediaRequest {
    pub media_item_ids: Vec<String>,
}
