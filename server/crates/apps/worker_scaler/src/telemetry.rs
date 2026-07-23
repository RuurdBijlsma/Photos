use sysinfo::System;

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub physical_cores: usize,
}

impl Telemetry {
    #[must_use]
    pub fn fetch() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();

        let total_memory_mb = sys.total_memory() / (1024 * 1024);
        let available_memory_mb = sys.available_memory() / (1024 * 1024);
        let physical_cores = sys.physical_core_count().unwrap_or(1);

        Self {
            total_memory_mb,
            available_memory_mb,
            physical_cores,
        }
    }

    /// Calculate the required memory buffer in MB based on settings.
    /// Buffer is `min(total_memory * buffer_pct, max_buffer_mb)`.
    #[must_use]
    pub fn required_memory_buffer_mb(&self, buffer_pct: f64, max_buffer_mb: u64) -> u64 {
        let pct_buffer = ((self.total_memory_mb as f64) * (buffer_pct / 100.0)) as u64;
        pct_buffer.min(max_buffer_mb)
    }

    /// Calculates available RAM headroom above the required buffer.
    #[must_use]
    pub fn memory_headroom_mb(&self, buffer_pct: f64, max_buffer_mb: u64) -> u64 {
        let required = self.required_memory_buffer_mb(buffer_pct, max_buffer_mb);
        self.available_memory_mb.saturating_sub(required)
    }
}
