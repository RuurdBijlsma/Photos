use app_state::AppSettings;
use color_eyre::Result;
use common_services::database::jobs::JobType;
use media_analyzer::MediaAnalyzer;
use ml_analysis::VisualAnalyzer;
use open_clip_inference::TextEmbedder;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;
use worker::models::WorkerModels;

struct ModelSlot<T> {
    name: String,
    instance: Option<Arc<T>>,
    idle_since: Option<Instant>,
    idle_timeout: Duration,
}

impl<T> ModelSlot<T> {
    fn new(name: &str, idle_timeout: Duration) -> Self {
        Self {
            name: name.to_owned(),
            instance: None,
            idle_since: None,
            idle_timeout,
        }
    }

    fn get(&mut self) -> Option<Arc<T>> {
        if let Some(ref arc) = self.instance {
            self.idle_since = None; // Reset idle timer when requested
            Some(arc.clone())
        } else {
            None
        }
    }

    fn set(&mut self, arc: Arc<T>) {
        self.idle_since = None;
        self.instance = Some(arc);
    }

    fn cleanup_idle(&mut self) {
        if let Some(ref arc) = self.instance {
            if Arc::strong_count(arc) > 1 {
                // At least one active worker is currently using this model
                self.idle_since = None;
            } else {
                // Model is only held by ModelRegistry (0 active workers using it)
                let idle_since = *self.idle_since.get_or_insert_with(Instant::now);
                if idle_since.elapsed() >= self.idle_timeout {
                    info!(
                        "🪶 Unloading {} after {:?} of inactivity.",
                        &self.name, self.idle_timeout
                    );
                    self.instance = None;
                    self.idle_since = None;
                }
            }
        }
    }
}

/// Manages shared ML model instances in `worker_scaler`.
pub struct ModelRegistry {
    settings: AppSettings,
    media_analyzer: ModelSlot<MediaAnalyzer>,
    visual_analyzer: ModelSlot<VisualAnalyzer>,
    text_embedder: ModelSlot<TextEmbedder>,
}

impl ModelRegistry {
    #[must_use]
    pub fn new(settings: AppSettings, idle_timeout: Duration) -> Self {
        Self {
            settings,
            media_analyzer: ModelSlot::new("MediaAnalyzer", idle_timeout),
            visual_analyzer: ModelSlot::new("VisualAnalyzer", idle_timeout),
            text_embedder: ModelSlot::new("TextEmbedder", idle_timeout),
        }
    }

    pub async fn get_or_load_media_analyzer(&mut self) -> Result<Arc<MediaAnalyzer>> {
        if let Some(analyzer) = self.media_analyzer.get() {
            return Ok(analyzer);
        }

        info!("Loading MediaAnalyzer...");
        let analyzer = Arc::new(MediaAnalyzer::builder().build().await?);
        self.media_analyzer.set(analyzer.clone());
        Ok(analyzer)
    }

    pub async fn get_or_load_visual_analyzer(&mut self) -> Result<Arc<VisualAnalyzer>> {
        if let Some(analyzer) = self.visual_analyzer.get() {
            return Ok(analyzer);
        }

        let embedder_model_id = &self.settings.ingest.analyzer.search.embedder_model_id;
        let cache_folder = &self.settings.ingest.hf_cache_root;

        info!("Loading VisualAnalyzer...");
        let analyzer = Arc::new(VisualAnalyzer::new(embedder_model_id, cache_folder).await?);
        self.visual_analyzer.set(analyzer.clone());
        Ok(analyzer)
    }

    pub async fn get_or_load_text_embedder(&mut self) -> Result<Arc<TextEmbedder>> {
        if let Some(embedder) = self.text_embedder.get() {
            return Ok(embedder);
        }

        let embedder_model_id = &self.settings.ingest.analyzer.search.embedder_model_id;
        let cache_folder = &self.settings.ingest.hf_cache_root;

        info!("Loading CLIP text embedder...");
        let embedder = TextEmbedder::from_hf(embedder_model_id)
            .cache_dir(cache_folder)
            .build()
            .await?;
        let embedder = Arc::new(embedder);
        self.text_embedder.set(embedder.clone());
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

    /// Checks for models unused by workers and unloads them if idle for too long
    pub fn cleanup_idle_models(&mut self) {
        self.media_analyzer.cleanup_idle();
        self.visual_analyzer.cleanup_idle();
        self.text_embedder.cleanup_idle();
    }
}
