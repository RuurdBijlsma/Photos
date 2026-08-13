use app_state::AppSettings;
use color_eyre::Result;
use common_services::database::jobs::JobType;
use media_analyzer::MediaAnalyzer;
use ml_analysis::VisualAnalyzer;
use open_clip_inference::TextEmbedder;
use std::sync::Arc;
use tracing::info;
use worker::models::WorkerModels;

/// Manages shared ML model instances in `worker_scaler` to share across worker contexts.
pub struct ModelRegistry {
    settings: AppSettings,
    media_analyzer: Option<Arc<MediaAnalyzer>>,
    visual_analyzer: Option<Arc<VisualAnalyzer>>,
    text_embedder: Option<Arc<TextEmbedder>>,
}

impl ModelRegistry {
    #[must_use]
    pub const fn new(settings: AppSettings) -> Self {
        Self {
            settings,
            media_analyzer: None,
            visual_analyzer: None,
            text_embedder: None,
        }
    }

    pub async fn get_or_load_media_analyzer(&mut self) -> Result<Arc<MediaAnalyzer>> {
        if let Some(ref analyzer) = self.media_analyzer {
            return Ok(analyzer.clone());
        }

        info!("Loading MediaAnalyzer in Scaler...");
        let analyzer = Arc::new(MediaAnalyzer::builder().build().await?);
        self.media_analyzer = Some(analyzer.clone());
        Ok(analyzer)
    }

    pub async fn get_or_load_visual_analyzer(&mut self) -> Result<Arc<VisualAnalyzer>> {
        if let Some(ref analyzer) = self.visual_analyzer {
            return Ok(analyzer.clone());
        }

        info!("Loading VisualAnalyzer in Scaler...");
        let analyzer = Arc::new(VisualAnalyzer::new(&self.settings.ingest.analyzer.search.embedder_model_id, &self.settings.ingest.hf_cache_root).await?);
        self.visual_analyzer = Some(analyzer.clone());
        Ok(analyzer)
    }

    pub async fn get_or_load_text_embedder(&mut self) -> Result<Arc<TextEmbedder>> {
        if let Some(ref embedder) = self.text_embedder {
            return Ok(embedder.clone());
        }

        info!("Loading CLIP text embedder in Scaler...");
        let embedder = TextEmbedder::from_hf(&self.settings.ingest.analyzer.search.embedder_model_id)
            .cache_dir(&self.settings.ingest.hf_cache_root)
            .build()
            .await?;
        let embedder = Arc::new(embedder);
        self.text_embedder = Some(embedder.clone());
        Ok(embedder)
    }

    /// Prepares the required `WorkerModels` bundle for a worker based on excluded job types.
    pub async fn prepare_models_for_jobs(
        &mut self,
        excluded_jobs: &[JobType],
    ) -> Result<WorkerModels> {
        let media_analyzer = self.get_or_load_media_analyzer().await?;

        let visual_analyzer = if !excluded_jobs.contains(&JobType::IngestLlm)
            || !excluded_jobs.contains(&JobType::IngestAnalysis)
        {
            Some(self.get_or_load_visual_analyzer().await?)
        } else {
            None
        };

        let text_embedder = if excluded_jobs.contains(&JobType::ClusterPhotos) {
            None
        } else {
            Some(self.get_or_load_text_embedder().await?)
        };

        Ok(WorkerModels::new(
            media_analyzer,
            visual_analyzer,
            text_embedder,
        ))
    }
}