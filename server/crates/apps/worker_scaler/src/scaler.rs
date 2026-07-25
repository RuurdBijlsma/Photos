use crate::demand::get_demand;
use crate::telemetry::Telemetry;
use app_state::{AppSettings, ProfileSettings};
use color_eyre::Result;
use common_services::database::jobs::JobType;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{Instrument, error, info, info_span, warn};
use worker::worker::create_worker_with_shutdown;

pub struct ActiveWorker {
    pub id: String,
    pub profile: String,
    pub spawned_at: Instant,
    pub shutdown_tx: watch::Sender<bool>,
    pub handle: JoinHandle<Result<()>>,
}

pub struct Scaler {
    pool: PgPool,
    settings: AppSettings,
    active_workers: Vec<ActiveWorker>,
    last_spawn_time: Option<Instant>,
    worker_counter: u64,
}

impl Scaler {
    #[must_use]
    pub const fn new(pool: PgPool, settings: AppSettings) -> Self {
        Self {
            pool,
            settings,
            active_workers: Vec::new(),
            last_spawn_time: None,
            worker_counter: 0,
        }
    }

    pub async fn run(&mut self, mut shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        let scaler_span = info_span!("scaler");
        self.run_inner(&mut shutdown_rx)
            .instrument(scaler_span)
            .await
    }

    async fn run_inner(&mut self, shutdown_rx: &mut watch::Receiver<bool>) -> Result<()> {
        info!("⚖️ Starting scaler...");
        let tick_duration = Duration::from_secs(self.settings.scaler.tick_interval_secs);

        loop {
            if *shutdown_rx.borrow() {
                self.shutdown_all_workers().await;
                break;
            }

            if let Err(e) = self.tick().await {
                error!("🚨 Error during scaler tick: {e}");
            }

            tokio::select! {
                () = tokio::time::sleep(tick_duration) => {}
                _ = shutdown_rx.changed() => {
                    self.shutdown_all_workers().await;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        self.clean_up_sleeping_workers();
        let demand = get_demand(&self.pool, &self.settings).await?;
        let telemetry = Telemetry::fetch();

        let headroom_mb = telemetry.memory_headroom_mb(&self.settings.scaler);
        let buffer_mb = telemetry.required_memory_buffer_mb(&self.settings.scaler);
        if telemetry.available_memory_mb < buffer_mb {
            warn!(
                "Available RAM ({} MB) dipped below buffer ({} MB). Initiating scale-down.",
                telemetry.available_memory_mb, buffer_mb
            );
            self.scale_down();
            return Ok(());
        }

        // stop if last worker spawn was too recent
        let cooldown = Duration::from_secs(self.settings.scaler.cooldown_period_secs);
        if let Some(last_spawn) = self.last_spawn_time
            && last_spawn.elapsed() < cooldown
        {
            return Ok(());
        }

        for profile in self.settings.scaler.profiles.clone() {
            let active_count = self.count_active(&profile.name);
            let max_count = profile.max_workers;
            let profile_demand = demand.get(&profile.name).copied().unwrap_or(0);

            if profile_demand > active_count {
                if headroom_mb < profile.estimated_ram_mb {
                    warn!(
                        "Demand for '{}' ({}) exists, but headroom ({} MB) is below estimated RAM ({} MB).",
                        profile.name, profile_demand, headroom_mb, profile.estimated_ram_mb
                    );
                } else if active_count < max_count {
                    self.spawn_worker(&profile);
                    self.last_spawn_time = Some(Instant::now());
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    fn clean_up_sleeping_workers(&mut self) {
        let mut removed_count = 0;
        let mut i = 0;
        while i < self.active_workers.len() {
            if self.active_workers[i].handle.is_finished() {
                let worker = self.active_workers.remove(i);
                info!(
                    "🧹 Worker finished: id={}, profile={:?}",
                    worker.id, worker.profile
                );
                removed_count += 1;
            } else {
                i += 1;
            }
        }

        if removed_count > 0 {
            self.log_worker_state_change(&format!(
                "[🚪] {} worker{} exited due to no active jobs",
                removed_count,
                if removed_count == 1 { "" } else { "s" }
            ));
        }
    }

    fn count_active(&self, profile_name: &str) -> usize {
        self.active_workers
            .iter()
            .filter(|w| w.profile == profile_name)
            .count()
    }

    fn spawn_worker(&mut self, profile: &ProfileSettings) {
        self.worker_counter += 1;
        let worker_id = format!("w{}-{}", self.worker_counter, profile.name);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let pool = self.pool.clone();
        let settings = self.settings.clone();

        let excluded_jobs: Vec<JobType> = profile
            .excluded_jobs
            .iter()
            .filter_map(|job_str| JobType::parse_from_str(job_str).ok())
            .collect();

        let worker_id_clone = worker_id.clone();
        let handle = tokio::spawn(async move {
            create_worker_with_shutdown(
                pool,
                settings,
                worker_id_clone,
                excluded_jobs,
                true,
                shutdown_rx,
            )
            .await
        });

        self.active_workers.push(ActiveWorker {
            id: worker_id,
            profile: profile.name.clone(),
            spawned_at: Instant::now(),
            shutdown_tx,
            handle,
        });

        self.log_worker_state_change(&format!("[➕ ] Spawned {} worker", profile.name));
    }

    fn scale_down(&self) {
        // Order profiles by priority ASC for scale-down (lowest priority scale-down first)
        let mut profiles = self.settings.scaler.profiles.clone();
        profiles.sort_by_key(|a| a.priority);

        for profile in profiles {
            if let Some(pos) = self
                .active_workers
                .iter()
                .position(|w| w.profile == profile.name)
            {
                let worker = &self.active_workers[pos];
                info!(
                    "🔫 Sending shutdown signal to worker: id={}, profile={}",
                    worker.id, worker.profile
                );
                let _ = worker.shutdown_tx.send(true);

                self.log_worker_state_change(&format!(
                    "[➖] Requested shutdown for worker {} ({})",
                    worker.id, worker.profile
                ));
                return;
            }
        }
    }

    async fn shutdown_all_workers(&mut self) {
        if self.active_workers.is_empty() {
            return;
        }
        info!(
            "🛑 Requesting graceful shutdown for all {} active workers...",
            self.active_workers.len()
        );
        for worker in &self.active_workers {
            let _ = worker.shutdown_tx.send(true);
        }
        for worker in self.active_workers.drain(..) {
            let _ = worker.handle.await;
        }
    }

    fn log_worker_state_change(&self, event_description: &str) {
        let profile_counts_str: Vec<String> = self
            .settings
            .scaler
            .profiles
            .iter()
            .map(|profile| {
                let count = self.count_active(&profile.name);
                format!("{}: {count}", profile.name)
            })
            .collect();

        let total = self.active_workers.len();

        info!(
            "{} | Total Active: {} [{}]",
            event_description,
            total,
            profile_counts_str.join(", ")
        );
    }
}
