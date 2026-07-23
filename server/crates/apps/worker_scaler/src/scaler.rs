use crate::config::ScalerConfig;
use crate::demand::query_queue_demand;
use crate::profile::WorkerProfile;
use crate::telemetry::Telemetry;
use app_state::AppSettings;
use color_eyre::Result;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use worker::worker::create_worker_with_shutdown;

pub struct ActiveWorker {
    pub id: String,
    pub profile: WorkerProfile,
    pub spawned_at: Instant,
    pub shutdown_tx: watch::Sender<bool>,
    pub handle: JoinHandle<Result<()>>,
}

pub struct Scaler {
    pool: PgPool,
    settings: AppSettings,
    config: ScalerConfig,
    active_workers: Vec<ActiveWorker>,
    last_spawn_time: Option<Instant>,
    worker_counter: u64,
}

impl Scaler {
    #[must_use]
    pub const fn new(pool: PgPool, settings: AppSettings, config: ScalerConfig) -> Self {
        Self {
            pool,
            settings,
            config,
            active_workers: Vec::new(),
            last_spawn_time: None,
            worker_counter: 0,
        }
    }

    pub async fn run(&mut self, mut shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        info!("🚀 [Worker Scaler] Starting control loop...");
        let tick_duration = Duration::from_secs(self.config.tick_interval_secs);

        loop {
            if *shutdown_rx.borrow() {
                info!("🛑 [Worker Scaler] Shutdown requested. Stopping scaler and all workers...");
                self.shutdown_all_workers().await;
                break;
            }

            if let Err(e) = self.tick().await {
                error!("⚠️ [Worker Scaler] Error during scaler tick: {e}");
            }

            tokio::select! {
                () = tokio::time::sleep(tick_duration) => {}
                _ = shutdown_rx.changed() => {
                    info!("🛑 [Worker Scaler] Shutdown signal received during sleep. Stopping scaler...");
                    self.shutdown_all_workers().await;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        // Step 1: Housekeeping - Reclaim finished tasks
        self.housekeeping();

        // Step 2: Query Runnable Queue Demand
        let demand = query_queue_demand(&self.pool).await?;

        // Step 3: Fetch System Telemetry
        let telemetry = Telemetry::fetch();

        // Step 4: Calculate Memory Headroom & Buffer
        let headroom_mb = telemetry.memory_headroom_mb(
            self.config.system_memory_buffer_percentage,
            self.config.system_memory_buffer_maximum_mb,
        );
        let buffer_mb = telemetry.required_memory_buffer_mb(
            self.config.system_memory_buffer_percentage,
            self.config.system_memory_buffer_maximum_mb,
        );

        // Step 5: Evaluate Scaling Actions
        // First check Scale-Down if available memory dips below buffer
        if telemetry.available_memory_mb < buffer_mb {
            warn!(
                "⚠️ [Worker Scaler] Available RAM ({} MB) dipped below buffer ({} MB). Initiating scale-down.",
                telemetry.available_memory_mb, buffer_mb
            );
            self.scale_down();
            return Ok(());
        }

        // Check Scale-Up Logic
        let cooldown = Duration::from_secs(self.config.cooldown_period_secs);
        if let Some(last_spawn) = self.last_spawn_time
            && last_spawn.elapsed() < cooldown {
                return Ok(());
            }

        // Try scaling up in order of priority: Heavy -> Medium -> Light -> Llm
        if self.try_spawn_profile(WorkerProfile::Heavy, demand.heavy_demand, headroom_mb) {
            return Ok(());
        }
        if self.try_spawn_profile(WorkerProfile::Medium, demand.medium_demand, headroom_mb) {
            return Ok(());
        }
        if self.try_spawn_profile(WorkerProfile::Light, demand.light_demand, headroom_mb) {
            return Ok(());
        }
        if self.try_spawn_profile(WorkerProfile::Llm, demand.llm_demand, headroom_mb) {
            return Ok(());
        }

        Ok(())
    }

    fn housekeeping(&mut self) {
        let mut i = 0;
        while i < self.active_workers.len() {
            if self.active_workers[i].handle.is_finished() {
                let worker = self.active_workers.remove(i);
                info!(
                    "🧹 [Worker Scaler] Reclaimed finished worker task: id={}, profile={:?}",
                    worker.id, worker.profile
                );
            } else {
                i += 1;
            }
        }
    }

    fn count_active(&self, profile: WorkerProfile) -> usize {
        self.active_workers
            .iter()
            .filter(|w| w.profile == profile)
            .count()
    }

    fn try_spawn_profile(
        &mut self,
        profile: WorkerProfile,
        demand: usize,
        headroom_mb: u64,
    ) -> bool {
        let active_count = self.count_active(profile);
        let max_count = self.config.max_workers(profile);

        if demand > active_count
            && active_count < max_count
            && headroom_mb >= profile.estimated_ram_mb()
        {
            self.spawn_worker(profile);
            self.last_spawn_time = Some(Instant::now());
            true
        } else {
            false
        }
    }

    fn spawn_worker(&mut self, profile: WorkerProfile) {
        self.worker_counter += 1;
        let worker_id = format!("scaler-w{}-{:?}", self.worker_counter, profile);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let pool = self.pool.clone();
        let settings = self.settings.clone();
        let excluded_jobs = profile.excluded_jobs();

        info!(
            "➕ [Worker Scaler] Spawning worker: id={}, profile={:?}, excluded_jobs={:?}",
            worker_id, profile, excluded_jobs
        );

        let handle = tokio::spawn(async move {
            create_worker_with_shutdown(pool, settings, excluded_jobs, false, shutdown_rx).await
        });

        self.active_workers.push(ActiveWorker {
            id: worker_id,
            profile,
            spawned_at: Instant::now(),
            shutdown_tx,
            handle,
        });
    }

    fn scale_down(&self) {
        // Find candidate to stop: prioritize Llm -> Heavy -> Medium -> Light
        let priority = [
            WorkerProfile::Llm,
            WorkerProfile::Heavy,
            WorkerProfile::Medium,
            WorkerProfile::Light,
        ];

        for &profile in &priority {
            if let Some(pos) = self
                .active_workers
                .iter()
                .position(|w| w.profile == profile)
            {
                let worker = &self.active_workers[pos];
                info!(
                    "📉 [Worker Scaler] Sending shutdown signal to worker: id={}, profile={:?}",
                    worker.id, worker.profile
                );
                let _ = worker.shutdown_tx.send(true);
                return;
            }
        }
    }

    async fn shutdown_all_workers(&mut self) {
        info!(
            "🛑 [Worker Scaler] Requesting graceful shutdown for all {} active workers...",
            self.active_workers.len()
        );
        for worker in &self.active_workers {
            let _ = worker.shutdown_tx.send(true);
        }
        for worker in self.active_workers.drain(..) {
            let _ = worker.handle.await;
        }
        info!("✅ [Worker Scaler] All workers stopped.");
    }
}
