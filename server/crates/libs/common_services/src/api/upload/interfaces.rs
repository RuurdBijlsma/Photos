use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TakeoutBatchPayload {
    pub tus_upload_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeoutStartResponse {
    pub started: bool,
    pub zip_count: usize,
}
