use serde::{Deserialize, Serialize};

// --- Admin Config ---

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GooglePhotosConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveGoogleConfigRequest {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GooglePhotosConfigStatus {
    pub configured: bool,
    pub client_id: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GooglePhotosUserStatus {
    pub configured: bool,
    pub client_id: Option<String>,
}

// --- Google Picker API response shapes ---

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PickerSession {
    pub id: String,
    pub picker_uri: String,
    pub media_items_set: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PickedMediaItem {
    pub id: String,
    pub media_file: Option<PickedMediaFile>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PickedMediaFile {
    pub filename: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PickedMediaItemsResponse {
    pub media_items: Option<Vec<PickedMediaItem>>,
    pub next_page_token: Option<String>,
}

// --- Our API request/response shapes ---

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeCodeRequest {
    pub code: String,
    pub redirect_uri: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StartPickerResponse {
    pub import_session_id: String,
    pub picker_uri: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchedItem {
    pub media_item_id: String,
    pub filename: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UnmatchedItem {
    pub filename: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PollResult {
    pub ready: bool,
    pub matched: Vec<MatchedItem>,
    pub unmatched: Vec<UnmatchedItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitImportRequest {
    pub import_session_id: String,
    pub album_name: String,
    pub album_description: Option<String>,
    pub media_item_ids: Vec<String>,
}

/// Internal OAuth token exchange response from Google
#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
}
