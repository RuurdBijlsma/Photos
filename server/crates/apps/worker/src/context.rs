use app_state::AppSettings;
use color_eyre::Result;
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
    /// Creates a new instance of `WorkerContext`.
    pub async fn new(
        pool: PgPool,
        settings: AppSettings,
        worker_id: String,
        excluded_job_types: Vec<JobType>,
    ) -> Result<Self> {
        let embedder_model_id = &settings.ingest.analyzer.search.embedder_model_id;

        // Load visual_analyzer IF the worker CAN run IngestLlm OR IngestAnalysis
        let visual_analyzer = if !excluded_job_types.contains(&JobType::IngestLlm)
            || !excluded_job_types.contains(&JobType::IngestAnalysis)
        {
            Some(Arc::new(VisualAnalyzer::new(embedder_model_id).await?))
        } else {
            None
        };

        // Load text_embedder IF the worker CAN run ClusterPhotos
        let text_embedder = if excluded_job_types.contains(&JobType::ClusterPhotos) {
            None
        } else {
            let embedder = TextEmbedder::from_hf(embedder_model_id).build().await?;
            Some(Arc::new(embedder))
        };

        Ok(Self {
            worker_id,
            excluded_job_types,
            pool,
            settings,
            media_analyzer: Arc::new(MediaAnalyzer::builder().build().await?),
            visual_analyzer,
            s2s_client: S2SClient::new(Client::new()),
            text_embedder,
        })
    }
}
