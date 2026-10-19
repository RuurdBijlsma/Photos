use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TakeoutBatchPayload {
    pub batch_id: String,
    pub tus_upload_ids: Vec<String>,
}