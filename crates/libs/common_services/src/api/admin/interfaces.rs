use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderParams {
    pub folder: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInfoResponse {
    pub folder: String,
    pub disk_available: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub read_access: bool,
    pub write_access: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSampleResponse {
    pub read_access: bool,
    pub folder: String,
    pub photo_count: usize,
    pub video_count: usize,
    pub samples: Vec<String>,
}

impl MediaSampleResponse {
    #[must_use]
    pub const fn unreadable(folder: String) -> Self {
        Self {
            read_access: false,
            folder,
            photo_count: 0,
            video_count: 0,
            samples: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedFilesResponse {
    pub read_access: bool,
    pub folder: String,
    pub inaccessible_entries: Vec<String>,
    pub unsupported_files: HashMap<String, Vec<String>>,
    pub unsupported_count: usize,
}

impl UnsupportedFilesResponse {
    #[must_use]
    pub fn unreadable(folder: String) -> Self {
        Self {
            read_access: false,
            folder,
            inaccessible_entries: Vec::new(),
            unsupported_files: HashMap::new(),
            unsupported_count: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskResponse {
    pub media_folder: PathInfoResponse,
    pub app_data_folder: PathInfoResponse,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MakeFolderBody {
    pub base_folder: String,
    pub new_name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserInfo {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub avatar_id: Option<String>,
    pub media_folder: Option<String>,
    pub main_drive_used: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserMediaFolderBody {
    pub user_folder: String,
}
