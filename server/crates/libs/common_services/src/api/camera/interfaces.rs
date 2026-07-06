use crate::database::UpdateField;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergePersonRequest {
    pub target_person_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePersonRequest {
    #[serde(default, skip_serializing_if = "UpdateField::is_ignore")]
    pub name: UpdateField<String>,
    #[serde(default, skip_serializing_if = "UpdateField::is_ignore")]
    pub face_thumb_id: UpdateField<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSummary {
    pub camera_make: String,
    pub camera_model: String,
    pub count: i32,
    pub thumbnail_id: String,
}
