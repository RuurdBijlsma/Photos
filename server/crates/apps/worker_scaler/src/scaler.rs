use crate::demand::get_demand;
use crate::telemetry::Telemetry;
use app_state::{AppSettings, ProfileSettings};
use color_eyre::Result;
use common_services::database::jobs::JobType;
use media_analyzer::MediaAnalyzer;
use model_provider::ModelProvider;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{Instrument, error, info, info_span, warn};
use worker::models::WorkerModels;
use worker::worker::create_worker;

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
    model_provider: Arc<ModelProvider>,
    active_workers: Vec<ActiveWorker>,
    last_spawn_time: Option<Instant>,
    worker_counter: u64,
}

impl Scaler {
    #[must_use]
    pub const fn new(pool: PgPool, settings: AppSettings, model_provider: Arc<ModelProvider>) -> Self {
        Self {
            pool,
            settings,
            model_provider,
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

        loop {
            if *shutdown_rx.borrow() {
                break;
            }

            let tick_delay = match self.tick().await {
                Ok(true) => self.settings.scaler.tick_interval,
                Ok(false) => self.settings.scaler.quick_tick_interval,
                Err(e) => {
                    error!("🚨 Error during scaler tick: {e}");
                    self.settings.scaler.tick_interval
                }
            };

            tokio::select! {
                () = tokio::time::sleep(tick_delay) => {}
                _ = shutdown_rx.changed() => break,
            }
        }

        self.shutdown_all_workers().await;

        Ok(())
    }

    /// Performs a tick and returns `true` if the system is completely idle.
    async fn tick(&mut self) -> Result<bool> {
        self.clean_up_sleeping_workers();
        self.model_provider.cleanup_idle_models().await;
        let demand = get_demand(&self.pool).await?;
        let telemetry = Telemetry::fetch();

        let headroom_mb = telemetry.memory_headroom_mb(&self.settings.scaler);
        let buffer_mb = telemetry.required_memory_buffer_mb(&self.settings.scaler);
        if telemetry.available_memory_mb < buffer_mb {
            warn!(
                "Available RAM ({} MB) dipped below buffer ({} MB). Initiating scale-down.",
                telemetry.available_memory_mb, buffer_mb
            );
            self.scale_down();
            return Ok(false);
        }

        let has_demand = !demand.is_empty() && demand.values().any(|&c| c > 0);
        let is_idle = !has_demand && self.active_workers.is_empty();

        if !has_demand {
            return Ok(is_idle);
        }

        let best_profile = self
            .settings
            .scaler
            .profiles
            .iter()
            .filter(|profile| {
                let active_count = self.count_active(&profile.name);
                if profile.max_workers == 0 || active_count >= profile.max_workers {
                    return false;
                }

                if headroom_mb < profile.estimated_ram_mb {
                    return false;
                }

                // Check if profile can handle at least one job type currently in demand
                demand.iter().any(|(&job_type, &count)| {
                    count > 0
                        && !profile.excluded_jobs.iter().any(|excluded| {
                        JobType::parse_from_str(excluded).ok() == Some(job_type)
                    })
                })
            })
            .max_by_key(|profile| profile.priority);

        if let Some(profile) = best_profile.cloned() {
            if let Err(e) = self.spawn_worker(&profile).await {
                error!("Failed to spawn worker for profile {}: {e}", profile.name);
            }
            self.last_spawn_time = Some(Instant::now());
        }
        Ok(is_idle)
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

    async fn spawn_worker(&mut self, profile: &ProfileSettings) -> Result<()> {
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

        let visual_analyzer = if excluded_jobs.contains(&JobType::IngestAnalysis) {
            None
        } else {
            Some(self.model_provider.get_or_load_visual_analyzer().await?)
        };

        let text_embedder = if excluded_jobs.contains(&JobType::ClusterPhotos) {
            None
        } else {
            Some(self.model_provider.get_or_load_text_embedder().await?)
        };

        let media_analyzer = Arc::new(MediaAnalyzer::builder().build().await?);

        let worker_models = WorkerModels::new(
            media_analyzer,
            visual_analyzer,
            text_embedder,
        );

        let worker_id_clone = worker_id.clone();
        let handle = tokio::spawn(async move {
            create_worker(
                pool,
                settings,
                worker_id_clone,
                excluded_jobs,
                worker_models,
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
        Ok(())
    }

    fn scale_down(&self) {
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