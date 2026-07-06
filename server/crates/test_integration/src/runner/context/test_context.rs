use crate::runner::context::context_utils::{
    copy_dir_recursive, create_test_database, create_test_settings, force_drop_db,
};
use app_state::{
    AppConstants, AppSettings, CONSTANTS, DATABASE_URL, load_constants_from_path,
    load_settings_from_path,
};
use color_eyre::eyre::{Result, eyre};
use reqwest::Client;
use sqlx::PgPool;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{error, info, warn};

pub fn init_test_constants(constants: AppConstants) {
    if CONSTANTS.set(constants).is_err() {
        info!("AppConstants were already initialized by another test.");
    }
    let test_db_url = "postgres://photos_user:dev-password@localhost/photos";
    if DATABASE_URL.set(test_db_url.to_owned()).is_err() {
        info!("AppConstants were already initialized by another test.");
    }
}

/// The main context for our integration tests.
#[allow(dead_code)]
pub struct TestContext {
    pub pool: PgPool,
    pub settings: AppSettings,
    pub http_client: Client,
    // Private fields for robust cleanup on Drop
    db_name: String,
    management_pool: PgPool,
    media_dir: TempDir,
    thumbnail_dir: TempDir,
    api_handle: JoinHandle<()>,
    worker_handle: JoinHandle<()>,
    ml_worker_handle: JoinHandle<()>,
    watcher_handle: JoinHandle<()>,
}

impl TestContext {
    /// Sets up the entire test environment, including a dedicated database and background services.
    pub async fn new() -> Result<Self> {
        info!("Setting up test environment...");

        // Load base settings to get initial database connection info
        let settings_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/settings.yaml")
            .canonicalize()?;
        let base_settings = load_settings_from_path(&settings_path, None)?;
        let test_constants = load_constants_from_path(&settings_path)?;
        init_test_constants(test_constants);

        // 1. Set up the dedicated test database
        let db_name = "test_db".to_owned();

        // 2. Generate the final settings for this test run
        let (settings, media_dir, thumbnail_dir, db_url) =
            create_test_settings(&db_name, &base_settings)?;
        let (main_pool, management_pool) = create_test_database(&db_url, &db_name).await?;

        // 2.5. Copy over test media
        let assets_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/media_dir");
        copy_dir_recursive(&assets_source_path, media_dir.path())?;

        // 3. Spawn application components as background tasks
        let (api_handle, worker_handle, ml_worker_handle, watcher_handle) =
            Self::spawn_services(&main_pool, &settings);

        // 4. Wait for the API to be ready to accept traffic
        let http_client = Client::new();
        Self::wait_for_healthy_api(&settings, &http_client).await?;

        info!("Test environment is ready.");
        Ok(Self {
            pool: main_pool,
            settings,
            http_client,
            db_name,
            management_pool,
            media_dir,
            thumbnail_dir,
            api_handle,
            worker_handle,
            ml_worker_handle,
            watcher_handle,
        })
    }

    /// Spawns the API, worker, and watcher services as background tokio tasks.
    fn spawn_services(
        pool: &PgPool,
        settings: &AppSettings,
    ) -> (
        JoinHandle<()>,
        JoinHandle<()>,
        JoinHandle<()>,
        JoinHandle<()>,
    ) {
        // Spawn API server
        let api_pool = pool.clone();
        let api_settings = settings.clone();
        let api_handle = tokio::spawn(async move {
            if let Err(e) = api::serve(api_pool, api_settings, false).await {
                error!("API server failed: {}", e);
            }
        });

        // Spawn Worker
        let worker_pool = pool.clone();
        let worker_settings = settings.clone();
        let worker_handle = tokio::spawn(async move {
            if let Err(e) =
                worker::worker::create_worker(worker_pool, worker_settings, false, false).await
            {
                error!("Worker failed: {}", e);
            }
        });

        // Spawn Worker 2
        let ml_worker_pool = pool.clone();
        let ml_worker_settings = settings.clone();
        let ml_worker_handle = tokio::spawn(async move {
            if let Err(e) =
                worker::worker::create_worker(ml_worker_pool, ml_worker_settings, true, false).await
            {
                error!("Worker failed: {}", e);
            }
        });

        // Spawn Watcher
        let watcher_pool = pool.clone();
        let watcher_settings = settings.ingest.clone();
        let watcher_handle = tokio::spawn(async move {
            if let Err(e) = watcher::watcher::start_watching(&watcher_pool, &watcher_settings).await
            {
                error!("Watcher failed: {}", e);
            }
        });

        (api_handle, worker_handle, ml_worker_handle, watcher_handle)
    }

    /// Polls the `/health` endpoint until it receives a successful response or times out.
    async fn wait_for_healthy_api(settings: &AppSettings, http_client: &Client) -> Result<()> {
        for attempt in 1..=20 {
            info!("Health check attempt {}...", attempt);
            let health_url = format!("{}/health", &settings.api.public_url);
            match http_client.get(&health_url).send().await {
                Ok(response) if response.status().is_success() => {
                    info!("API is healthy!");
                    return Ok(());
                }
                Ok(response) => {
                    warn!(
                        "API health check returned non-success status: {}",
                        response.status()
                    );
                }
                Err(e) => {
                    warn!("API health check failed: {:?}. Retrying...", e);
                }
            }
            sleep(Duration::from_millis(500)).await;
        }
        Err(eyre!(
            "API did not become healthy within the timeout period."
        ))
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Abort background tasks
        self.api_handle.abort();
        self.worker_handle.abort();
        self.ml_worker_handle.abort();
        self.watcher_handle.abort();

        // Drop the test database
        let db_name = self.db_name.clone();
        let pool = self.management_pool.clone();
        tokio::spawn(async move {
            info!("Dropping test database: {}", db_name);
            force_drop_db(&pool, &db_name)
                .await
                .expect("Failed to clean up DB.");
        });

        info!("Teardown complete.");
    }
}
