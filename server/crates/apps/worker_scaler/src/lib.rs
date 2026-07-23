#![deny(clippy::unwrap_used)]
#![allow(
    clippy::cognitive_complexity,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::missing_panics_doc
)]

pub mod config;
pub mod demand;
pub mod profile;
pub mod scaler;
pub mod telemetry;

use app_state::AppSettings;
use color_eyre::Result;
use config::ScalerConfig;
use scaler::Scaler;
use sqlx::PgPool;
use tokio::sync::watch;

pub async fn start_scaler(
    pool: PgPool,
    settings: AppSettings,
    config: ScalerConfig,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let mut scaler = Scaler::new(pool, settings, config);
    scaler.run(shutdown_rx).await
}
