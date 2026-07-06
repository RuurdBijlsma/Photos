use chrono::{DateTime, Utc};
use serde::Serialize;

/// Represents the link between a media item and an album.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMediaItem {
    pub album_id: String,
    pub media_item_id: String,
    pub added_by_user: Option<i32>,
    pub added_at: DateTime<Utc>,
    pub rank: f64,
}
