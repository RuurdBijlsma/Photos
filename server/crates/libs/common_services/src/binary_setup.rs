use crate::database::get_db_pool;
use crate::logging::{LoggingGuard, init_logging};
use app_state::{AppSettings, database_url, load_app_settings};
use sqlx::PgPool;

/// Loads settings, installs the global subscriber and opens the database pool.
///
/// The returned [`LoggingGuard`] must be held for the lifetime of `main`: dropping
/// it stops the background log writer.
pub async fn setup_binary() -> color_eyre::Result<(AppSettings, PgPool, LoggingGuard)> {
    color_eyre::install()?;
    // Settings come first: the file sink needs `ingest.logs_root`, which only exists
    // once the settings are resolved.
    let settings = load_app_settings()?;
    let logging_guard = init_logging(&settings);
    let pool = get_db_pool(database_url(), true).await?;
    Ok((settings, pool, logging_guard))
}
