#![deny(clippy::unwrap_used)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]
extern crate core;

pub mod model_slot;

use app_state::AppSettings;
use color_eyre::Result;
use ml_analysis::VisualAnalyzer;
use model_slot::ModelSlot;
use open_clip_inference::{TextEmbedder, VisionEmbedder};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[derive(Debug)]
pub struct ModelProvider {
    settings: AppSettings,
    visual_analyzer: ModelSlot<VisualAnalyzer>,
    text_embedder: ModelSlot<TextEmbedder>,
}

impl ModelProvider {
    #[must_use]
    pub fn new(settings: AppSettings, idle_timeout: Duration) -> Self {
        Self {
            settings,
            visual_analyzer: ModelSlot::new("VisualAnalyzer", idle_timeout),
            text_embedder: ModelSlot::new("TextEmbedder", idle_timeout),
        }
    }

    pub async fn get_or_load_visual_analyzer(&self) -> Result<Arc<VisualAnalyzer>> {
        self.visual_analyzer
            .get_or_load(|| async {
                info!("Loading VisualAnalyzer...");
                let analyzer = VisualAnalyzer::new(&self.settings.ingest).await?;
                Ok(Arc::new(analyzer))
            })
            .await
    }

    pub async fn get_or_load_text_embedder(&self) -> Result<Arc<TextEmbedder>> {
        self.text_embedder
            .get_or_load(|| async {
                let embedder_model_id =
                    &self.settings.ingest.analyzer.search.embedder_model_id;

                info!("Loading CLIP text embedder...");
                let embedder = TextEmbedder::from_hf(embedder_model_id)
                    .cache_dir(&self.settings.ingest.hf_cache_root)
                    .with_inter_threads(self.settings.ingest.analyzer.onnx.inter_threads)
                    .with_intra_threads(self.settings.ingest.analyzer.onnx.intra_threads)
                    .build()
                    .await?;
                Ok(Arc::new(embedder))
            })
            .await
    }

    pub async fn get_or_load_vision_embedder(&self) -> Result<Arc<VisionEmbedder>> {
        let visual_analyzer = self.get_or_load_visual_analyzer().await?;
        Ok(visual_analyzer.embedder.clone())
    }

    pub async fn cleanup_idle_models(&self) {
        self.visual_analyzer.cleanup_idle().await;
        self.text_embedder.cleanup_idle().await;
    }
}