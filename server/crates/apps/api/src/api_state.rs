use app_state::{AppSettings, IngestSettings};
use axum::extract::FromRef;
use common_services::s2s_client::S2SClient;
use ml_analysis::VisualAnalyzer;
use model_provider::ModelProvider;
use open_clip_inference::{TextEmbedder, VisionEmbedder};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ApiContext {
    pub pool: PgPool,
    pub s2s_client: S2SClient,
    pub settings: AppSettings,
    pub model_provider: Arc<ModelProvider>,
}

impl ApiContext {
    pub async fn text_embedder(&self) -> color_eyre::Result<Arc<TextEmbedder>> {
        self.model_provider.get_or_load_text_embedder().await
    }

    pub async fn vision_embedder(&self) -> color_eyre::Result<Arc<VisionEmbedder>> {
        self.model_provider.get_or_load_vision_embedder().await
    }

    pub async fn visual_analyzer(&self) -> color_eyre::Result<Arc<VisualAnalyzer>> {
        self.model_provider.get_or_load_visual_analyzer().await
    }
}

impl FromRef<ApiContext> for PgPool {
    fn from_ref(state: &ApiContext) -> Self {
        state.pool.clone()
    }
}

impl FromRef<ApiContext> for S2SClient {
    fn from_ref(state: &ApiContext) -> Self {
        state.s2s_client.clone()
    }
}

impl FromRef<ApiContext> for AppSettings {
    fn from_ref(state: &ApiContext) -> Self {
        state.settings.clone()
    }
}

impl FromRef<ApiContext> for IngestSettings {
    fn from_ref(state: &ApiContext) -> Self {
        state.settings.ingest.clone()
    }
}

impl FromRef<ApiContext> for Arc<ModelProvider> {
    fn from_ref(state: &ApiContext) -> Self {
        state.model_provider.clone()
    }
}
