use crate::api::album::interfaces::AlbumSort;
use crate::database::UpdateField;
use crate::database::app_user::UserRole;
use chrono::{DateTime, NaiveDateTime, Utc};

pub struct UpdateUserPayload {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
    pub media_folder: Option<String>,
    pub avatar_id: UpdateField<String>,
}

pub struct UpdateMediaItemPayload {
    pub user_caption: UpdateField<String>,
    pub taken_at_local: Option<NaiveDateTime>,
    pub taken_at_utc: UpdateField<DateTime<Utc>>,
    pub sort_timestamp: Option<DateTime<Utc>>,
    pub timezone_offset_seconds: UpdateField<i32>,
    pub use_panorama_viewer: Option<bool>,
}

pub struct CreateAlbumPayload {
    pub owner_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub thumbnail_id: Option<String>,
    pub sort_mode: AlbumSort,
    pub is_public: bool,
}
