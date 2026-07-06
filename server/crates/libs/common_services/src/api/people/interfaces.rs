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

#[derive(Debug, Clone)]
pub struct PersonSummary {
    pub id: String,
    pub name: Option<String>,
    pub face_thumb_id: Option<String>,
    pub face_cluster_ids: Vec<String>,
    pub photo_count: i32,
}
