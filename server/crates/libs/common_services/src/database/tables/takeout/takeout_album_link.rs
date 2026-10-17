use serde::{Deserialize, Serialize};

/// One (album, filename) occurrence found in a takeout export.
///
/// While `media_item_id` is `NULL` the ingest of the single kept copy of that
/// filename has not happened yet; `IngestMetadata` consumes these rows in the
/// same transaction that inserts the media item, so album membership is
/// attached atomically no matter which zip arrived or was processed first.
/// Consumed rows stay behind (with the item id) as the record of "this
/// filename was seen in this album during takeout import".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeoutAlbumLink {
    pub id: i64,
    pub user_id: i32,
    pub batch_id: String,
    pub album_id: String,
    pub filename: String,
    pub media_item_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
