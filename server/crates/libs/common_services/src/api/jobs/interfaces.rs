use crate::database::jobs::{JobStatus, JobType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: i64,
    pub relative_path: Option<String>,
    pub user_id: Option<i32>,
    pub job_type: JobType,
    pub payload: Option<Value>,
    pub priority: i32,
    pub status: JobStatus,
    pub attempts: i32,
    pub dependency_attempts: i32,
    pub max_attempts: i32,
    pub owner: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub last_error: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobsQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,

    /// Sorting params, e.g. `sort=priority:asc&sort=scheduledAt:desc`
    #[serde(default)]
    pub sort: Vec<String>,

    /// Filter params, e.g. `filter=status:eq:queued&filter=priority:gte:100`
    #[serde(default)]
    pub filter: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedJobsResponse {
    pub data: Vec<JobInfo>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct IngestJobCounts {
    pub queued: i64,
    pub running: i64,
    pub failed: i64,
    pub done: i64,
    pub cancelled: i64,
    pub total: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct IngestOverviewResponse {
    pub metadata: IngestJobCounts,
    pub thumbnails: IngestJobCounts,
    pub analysis: IngestJobCounts,
    pub llm: IngestJobCounts,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RetryJobPayload {
    pub id: i64,
}
