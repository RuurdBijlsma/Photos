use serde::Serialize;

/// Represents a temporary record for a media item downloaded from another server,
/// awaiting ingestion.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAlbumMediaItem {
    pub relative_path: String,
    pub album_id: String,
    pub remote_user_identity: String,
}
