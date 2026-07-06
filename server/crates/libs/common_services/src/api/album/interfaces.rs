use crate::api::timeline::interfaces::SortDirection;
use crate::database::UpdateField;
use crate::database::album::album::AlbumRole;
use crate::database::media_item::camera_settings::CameraSettings;
use crate::database::media_item::media_item::FullMediaItem;
use crate::database::media_item::time_details::TimeDetails;
use crate::database::media_item::weather::Weather;
use crate::database::visual_analysis::visual_analysis::ReadVisualAnalysis;
use chrono::{DateTime, NaiveDateTime, Utc};
use common_types::pb::api::{CollaboratorSummary, TimelineItem};
use serde::{Deserialize, Serialize};
use sqlx::Type;
// --- Request Payloads ---

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "album_sort", rename_all = "snake_case")]
pub enum AlbumSort {
    DateAsc,
    DateDesc,
    AddedAsc,
    AddedDesc,
    None,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetSortedAlbumItemsRequest {
    pub sort_mode: AlbumSort,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlbumRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub media_item_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReorderMediaRequest {
    pub media_item_ids: Vec<String>,
    pub sort_mode: AlbumSort,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddMediaToAlbumRequest {
    pub media_item_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AddCollaboratorRequest {
    pub user_id: i32,
    pub role: AlbumRole,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlbumRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "UpdateField::is_ignore")]
    pub description: UpdateField<String>,
    #[serde(default, skip_serializing_if = "UpdateField::is_ignore")]
    pub thumbnail_id: UpdateField<String>,
    #[serde(default)]
    pub is_public: Option<bool>,
}

// --- Request Payloads for Cross-Server Sharing ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckInviteRequest {
    /// The full invitation token string (e.g., "inv-...")
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptInviteRequest {
    /// The full invitation token string.
    pub token: String,
    /// The name for the new album on the local server, pre-filled but editable by the user.
    pub name: String,
    /// The description for the new album on the local server.
    pub description: Option<String>,
}

// --- URL/Path Parameters ---

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumIdParams {
    pub album_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMediaParams {
    pub album_id: String,
    pub media_item_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveCollaboratorParams {
    pub album_id: String,
    pub collaborator_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAlbumsParam {
    #[serde(default)]
    pub sort_direction: SortDirection,
    #[serde(default)]
    pub sort_field: AlbumSortField,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GetAlbumMediaParams {
    /// Comma separated list of Rank IDs (Start Ranks of the groups).
    pub groups: String,
}

// --- Response Payloads ---

/// Full details of an album, including its media items and collaborators.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetailsResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub thumbnail_id: Option<String>,
    pub is_public: bool,
    pub owner_id: i32,
    pub created_at: DateTime<Utc>,
    pub media_items: Vec<AlbumMediaItemSummary>,
    pub collaborators: Vec<CollaboratorSummary>,
}

/// A summary of a media item within an album.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMediaItemSummary {
    pub media_item: TimelineItem,
    pub added_at: DateTime<Utc>,
}

/// Album reference for a media item detail response (info panel).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItemAlbumRef {
    pub id: String,
    pub name: String,
    pub thumbnail_id: Option<String>,
    pub media_count: i32,
}

/// Full media item detail with albums that contain it.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItemWithAlbums {
    pub media_item: FullMediaItem,
    pub albums: Vec<MediaItemAlbumRef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumShareClaims {
    pub iss: String, // Issuer (server's public_url)
    pub sub: String, // Subject (album_id)
    pub exp: i64,    // Expiration time (as a Unix timestamp)
    pub sharer_username: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AlbumSortField {
    #[default]
    UpdatedAt,
    LatestPhoto,
    Name,
}

impl AlbumSortField {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::UpdatedAt => "updated_at",
            Self::LatestPhoto => "latest_photo",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SharedMediaFeatures {
    pub mime_type: String,
    pub size_bytes: i64,
    pub is_motion_photo: bool,
    pub motion_photo_presentation_timestamp: Option<i64>,
    pub is_hdr: bool,
    pub is_burst: bool,
    pub burst_id: Option<String>,
    pub capture_fps: Option<f32>,
    pub video_fps: Option<f32>,
    pub is_nightsight: bool,
    pub is_timelapse: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SharedMediaItem {
    pub id: String,
    pub filename: String,
    pub width: i32,
    pub height: i32,
    pub is_video: bool,
    pub duration_ms: Option<i64>,
    pub taken_at_local: NaiveDateTime,
    pub taken_at_utc: Option<DateTime<Utc>>,
    pub timezone_name: Option<String>,
    pub timezone_offset_seconds: Option<i32>,
    pub use_panorama_viewer: bool,
    pub has_thumbnails: bool,
    pub visual_analyses: Vec<ReadVisualAnalysis>,
    pub time: TimeDetails,
    pub weather: Option<Weather>,
    pub media_features: SharedMediaFeatures,
    pub camera_settings: CameraSettings,
    pub user_caption: Option<String>,
}

impl From<FullMediaItem> for SharedMediaItem {
    fn from(item: FullMediaItem) -> Self {
        Self {
            id: item.id,
            filename: item.filename,
            width: item.width,
            height: item.height,
            is_video: item.is_video,
            duration_ms: item.duration_ms,
            taken_at_local: item.taken_at_local,
            taken_at_utc: item.taken_at_utc,
            timezone_name: item.timezone_name,
            timezone_offset_seconds: item.timezone_offset_seconds,
            use_panorama_viewer: item.use_panorama_viewer,
            has_thumbnails: item.has_thumbnails,
            visual_analyses: item.visual_analyses,
            time: item.time,
            weather: item.weather,
            camera_settings: item.camera_settings,
            user_caption: item.user_caption,
            media_features: SharedMediaFeatures {
                mime_type: item.media_features.mime_type,
                size_bytes: item.media_features.size_bytes,
                is_motion_photo: item.media_features.is_motion_photo,
                motion_photo_presentation_timestamp: item
                    .media_features
                    .motion_photo_presentation_timestamp,
                is_hdr: item.media_features.is_hdr,
                is_burst: item.media_features.is_burst,
                burst_id: item.media_features.burst_id,
                capture_fps: item.media_features.capture_fps,
                video_fps: item.media_features.video_fps,
                is_nightsight: item.media_features.is_nightsight,
                is_timelapse: item.media_features.is_timelapse,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AlbumItemBackup {
    pub hash: String,
    pub rank: f64,
    pub added_at: DateTime<Utc>,
    pub added_by_user: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AlbumBackup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub items: Vec<AlbumItemBackup>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}
