use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct PruneMissingRequest {
    pub ids: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PruneMissingResponse {
    pub pruned_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub disk_available: u64,
    pub disk_used: u64,
    pub disk_total: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiskStats {
    pub app_data_drive: DiskInfo,
    pub media_drive: DiskInfo,
    pub are_same_drive: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub is_ingesting: bool,
    pub has_clustered_people: bool,
    pub has_clustered_photos: bool,
    pub allow_file_modifications: bool,
    pub allow_file_deletion: bool,
    pub disk: DiskStats,
    pub media_folder_available: bool,
    pub media_folder_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublicSystemStats {
    pub has_users: bool,
}
