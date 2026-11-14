use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckHashesRequest {
    /// BLAKE3 lowercase hex digests.
    pub hashes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckHashesResponse {
    /// Requested hash -> the non-deleted media item ids that have it.
    pub hashes: HashMap<String, Vec<String>>,
}
