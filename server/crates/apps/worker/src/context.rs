use crate::models::WorkerModels;
use app_state::AppSettings;
use common_services::database::jobs::JobType;
use common_services::s2s_client::S2SClient;
use media_analyzer::MediaAnalyzer;
use ml_analysis::VisualAnalyzer;
use open_clip_inference::TextEmbedder;
use reqwest::Client;
use sqlx::PgPool;
use std::sync::Arc;

pub struct WorkerContext {
    pub worker_id: String,
    pub excluded_job_types: Vec<JobType>,
    pub pool: PgPool,
    pub settings: AppSettings,
    pub media_analyzer: Arc<MediaAnalyzer>,
    pub visual_analyzer: Option<Arc<VisualAnalyzer>>,
    pub text_embedder: Option<Arc<TextEmbedder>>,
    pub s2s_client: S2SClient,
}

impl WorkerContext {
    /// Creates a new instance of `WorkerContext` using provided models.
    #[must_use]
    pub fn new(
        pool: PgPool,
        settings: AppSettings,
        worker_id: String,
        excluded_job_types: Vec<JobType>,
        models: WorkerModels,
    ) -> Self {
        Self {
            worker_id,
            excluded_job_types,
            pool,
            settings,
            media_analyzer: models.media_analyzer,
            visual_analyzer: models.visual_analyzer,
            text_embedder: models.text_embedder,
            s2s_client: S2SClient::new(Client::new()),
        }
    }
}