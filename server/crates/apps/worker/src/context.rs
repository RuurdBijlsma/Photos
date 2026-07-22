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
    pub visual_analyzer: Arc<VisualAnalyzer>,
    pub text_embedder: Arc<TextEmbedder>,
    pub s2s_client: S2SClient,
}

impl WorkerContext {
    /// Creates a new instance of `WorkerContext`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the creation of `MediaAnalyzer` or `VisualAnalyzer` fails.
    pub async fn new(
        pool: PgPool,
        settings: AppSettings,
        worker_id: String,
        excluded_job_types: Vec<JobType>,
    ) -> Result<Self> {
        let embedder_model_id = &settings.ingest.analyzer.search.embedder_model_id.clone();
        let text_embedder =
            TextEmbedder::from_hf(&settings.ingest.analyzer.search.embedder_model_id)
                .build()
                .await?;
        Ok(Self {
            worker_id,
            excluded_job_types,
            pool,
            settings,
            media_analyzer: Arc::new(MediaAnalyzer::builder().build().await?),
            visual_analyzer: Arc::new(VisualAnalyzer::new(embedder_model_id).await?),
            s2s_client: S2SClient::new(Client::new()),
            text_embedder: Arc::new(text_embedder),
        })
    }
}