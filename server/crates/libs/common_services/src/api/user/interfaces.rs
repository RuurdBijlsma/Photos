use crate::database::UpdateField;
use crate::database::app_user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub photo_count: i64,
    pub video_count: i64,
    pub album_count: i64,
    pub shared_album_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub avatar_id: Option<String>,
    pub stats: UserStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserProfileRequest {
    pub name: Option<String>,
    pub avatar_id: UpdateField<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmallUser {
    pub id: i32,
    pub name: String,
    pub avatar_id: Option<String>,
}

impl From<&User> for SmallUser {
    fn from(value: &User) -> Self {
        Self {
            name: value.name.clone(),
            id: value.id,
            avatar_id: value.avatar_id.clone(),
        }
    }
}

impl From<User> for SmallUser {
    fn from(value: User) -> Self {
        (&value).into()
    }
}
