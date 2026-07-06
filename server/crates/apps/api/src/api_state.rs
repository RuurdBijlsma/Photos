use crate::timeline::websocket::MediaPayload;
use app_state::{AppSettings, IngestSettings};
use axum::extract::FromRef;
use common_services::s2s_client::S2SClient;
use open_clip_inference::{TextEmbedder, VisionEmbedder};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct ApiContext {
    pub pool: PgPool,
    pub s2s_client: S2SClient,
    pub settings: AppSettings,
    pub timeline_broadcaster: broadcast::Sender<Arc<MediaPayload>>,
    pub text_embedder: Arc<TextEmbedder>,
    pub vision_embedder: Arc<VisionEmbedder>,
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
