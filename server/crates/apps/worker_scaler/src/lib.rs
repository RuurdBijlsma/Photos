#![deny(clippy::unwrap_used)]
#![allow(
    clippy::cognitive_complexity,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::missing_panics_doc,
    clippy::large_futures
)]

pub mod demand;
pub mod scaler;
pub mod telemetry;

use app_state::AppSettings;
use color_eyre::Result;
use model_provider::ModelProvider;
use scaler::Scaler;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::watch;

pub async fn start_scaler(
    pool: PgPool,
    settings: AppSettings,
    model_provider: Arc<ModelProvider>,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let mut scaler = Scaler::new(pool, settings, model_provider);
    scaler.run(shutdown_rx).await
}
