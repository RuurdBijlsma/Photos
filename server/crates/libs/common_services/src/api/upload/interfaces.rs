use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadCompletePayload {
    pub upload_id: String,
    pub filename: String,
}