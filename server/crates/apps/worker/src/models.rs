use app_state::AppSettings;
use color_eyre::Result;
use common_services::database::jobs::JobType;
use media_analyzer::MediaAnalyzer;
use ml_analysis::VisualAnalyzer;
use open_clip_inference::TextEmbedder;
use std::sync::Arc;
use tracing::info;

/// Container holding shared ML models and analysis context for workers.
#[derive(Clone)]
pub struct WorkerModels {
    pub media_analyzer: Arc<MediaAnalyzer>,
    pub visual_analyzer: Option<Arc<VisualAnalyzer>>,
    pub text_embedder: Option<Arc<TextEmbedder>>,
}

impl WorkerModels {
    #[must_use]
    pub const fn new(
        media_analyzer: Arc<MediaAnalyzer>,
        visual_analyzer: Option<Arc<VisualAnalyzer>>,
        text_embedder: Option<Arc<TextEmbedder>>,
    ) -> Self {
        Self {
            media_analyzer,
            visual_analyzer,
            text_embedder,
        }
    }

    /// Helper to load required models for a worker based on excluded job types.
    /// Used in worker/main.rs only
    pub async fn load_for_jobs(
        settings: &AppSettings,
        excluded_job_types: &[JobType],
    ) -> Result<Self> {
        let embedder_model_id = &settings.ingest.analyzer.search.embedder_model_id;

        let media_analyzer = Arc::new(MediaAnalyzer::builder().build().await?);

        // Load visual_analyzer IF the worker CAN run IngestLlm OR IngestAnalysis
        let visual_analyzer = if excluded_job_types.contains(&JobType::IngestAnalysis) {
            None
        } else {
            Some(Arc::new(
                VisualAnalyzer::new(embedder_model_id, &settings.ingest.hf_cache_root).await?,
            ))
        };

        // Load text_embedder IF the worker CAN run ClusterPhotos
        let text_embedder = if excluded_job_types.contains(&JobType::ClusterPhotos) {
            None
        } else {
            info!("Loading CLIP text embedder...");
            let embedder = TextEmbedder::from_hf(embedder_model_id)
                .cache_dir(&settings.ingest.hf_cache_root)
                .build()
                .await?;
            Some(Arc::new(embedder))
        };

        Ok(Self {
            media_analyzer,
            visual_analyzer,
            text_embedder,
        })
    }
}
