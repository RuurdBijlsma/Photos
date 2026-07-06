use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAlbumPayload {
    pub album_name: String,
    pub album_description: Option<String>,
    pub remote_username: String,
    pub remote_url: Url,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAlbumItemPayload {
    pub remote_relative_path: String,
    pub local_album_id: String,
    pub remote_username: String,
    pub remote_url: Url,
    pub token: String,
}
