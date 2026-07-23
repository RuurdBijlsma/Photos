use common_services::database::jobs::JobType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerProfile {
    Light,
    Medium,
    Heavy,
    Llm,
}

impl WorkerProfile {
    /// Returns the jobs that this worker profile MUST NOT run.
    #[must_use]
    pub fn excluded_jobs(&self) -> Vec<JobType> {
        match self {
            Self::Light => vec![
                JobType::IngestThumbnails,
                JobType::IngestAnalysis,
                JobType::IngestLlm,
                JobType::ClusterFaces,
                JobType::ClusterPhotos,
            ],
            Self::Medium => vec![JobType::IngestLlm, JobType::IngestThumbnails],
            Self::Heavy => vec![JobType::IngestLlm],
            Self::Llm => vec![],
        }
    }

    /// Estimated RAM footprint when initialized (in Megabytes).
    #[must_use]
    pub const fn estimated_ram_mb(&self) -> u64 {
        match self {
            Self::Light => 100,
            Self::Medium => 500,
            Self::Heavy => 1000,
            Self::Llm => 6000,
        }
    }

    /// Default max worker count per profile.
    #[must_use]
    pub const fn default_max_workers(&self) -> usize {
        match self {
            Self::Light => 10,
            Self::Medium => 5,
            Self::Heavy => 2,
            Self::Llm => 0,
        }
    }
}
