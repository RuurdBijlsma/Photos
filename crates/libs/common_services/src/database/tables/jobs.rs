use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Type;

#[derive(Debug)]
#[allow(clippy::struct_field_names)]
pub struct Job {
    pub id: i64,
    pub payload: Option<Value>,
    pub relative_path: Option<String>,
    pub user_id: Option<i32>,
    pub job_type: JobType,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub dependency_attempts: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "job_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    IngestMetadata,
    IngestThumbnails,
    IngestAnalysis,
    IngestLlm,
    Remove,
    Scan,
    CleanDb,
    DelayedScan,
    ClusterFaces,
    ClusterPhotos,
    SyncThumbnails,
    ImportAlbumItem,
    UpdateGlobalCentroid,
    CalcSystemStats,
    GenerateDailyCards,
}

impl JobType {
    #[must_use]
    pub const fn get_priority(&self, is_video: bool) -> i32 {
        match self {
            Self::IngestMetadata => 50,
            Self::IngestThumbnails => {
                if is_video {
                    65
                } else {
                    60
                }
            }
            Self::IngestAnalysis => {
                if is_video {
                    95
                } else {
                    90
                }
            }
            Self::IngestLlm => {
                if is_video {
                    250
                } else {
                    240
                }
            }
            Self::Remove => 0,
            Self::CleanDb => 20,
            Self::ImportAlbumItem => 25,
            Self::Scan => 70,
            // These can be done after ingest analysis is done
            Self::UpdateGlobalCentroid => 100,
            Self::CalcSystemStats => 110,
            Self::ClusterFaces => 120,
            Self::ClusterPhotos => 130,
            Self::SyncThumbnails => 140,
            Self::DelayedScan => 150,
            Self::GenerateDailyCards => 160,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "job_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Failed,
    Done,
    Cancelled,
}
