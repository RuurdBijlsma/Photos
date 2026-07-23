use crate::profile::WorkerProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalerConfig {
    pub tick_interval_secs: u64,
    pub cooldown_period_secs: u64,
    pub system_memory_buffer_percentage: f64,
    pub system_memory_buffer_maximum_mb: u64,
    pub max_light_workers: usize,
    pub max_medium_workers: usize,
    pub max_heavy_workers: usize,
    pub max_llm_workers: usize,
}

impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            tick_interval_secs: 5,
            cooldown_period_secs: 20,
            system_memory_buffer_percentage: 10.0,
            system_memory_buffer_maximum_mb: 2000,
            max_light_workers: WorkerProfile::Light.default_max_workers(),
            max_medium_workers: WorkerProfile::Medium.default_max_workers(),
            max_heavy_workers: WorkerProfile::Heavy.default_max_workers(),
            max_llm_workers: WorkerProfile::Llm.default_max_workers(),
        }
    }
}

impl ScalerConfig {
    #[must_use]
    pub const fn max_workers(&self, profile: WorkerProfile) -> usize {
        match profile {
            WorkerProfile::Light => self.max_light_workers,
            WorkerProfile::Medium => self.max_medium_workers,
            WorkerProfile::Heavy => self.max_heavy_workers,
            WorkerProfile::Llm => self.max_llm_workers,
        }
    }
}
