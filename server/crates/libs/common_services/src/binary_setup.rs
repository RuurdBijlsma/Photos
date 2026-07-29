use crate::database::get_db_pool;
use app_state::{AppSettings, database_url, load_app_settings};
use sqlx::PgPool;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt, reload};

pub async fn setup_binary() -> color_eyre::Result<(AppSettings, PgPool)> {
    color_eyre::install()?;
    let initial_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let (filter_layer, reload_handle) = reload::Layer::new(initial_filter);
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer())
        .init();
    let settings = load_app_settings()?;
    if let Ok(config_filter) = EnvFilter::try_new(settings.logging.filters.to_filter_string()) {
        let _ = reload_handle.modify(|filter| *filter = config_filter);
    }
    let pool = get_db_pool(database_url(), true).await?;
    Ok((settings, pool))
}
