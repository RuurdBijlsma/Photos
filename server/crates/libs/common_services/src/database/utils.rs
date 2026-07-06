use app_state::constants;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use color_eyre::eyre::Result;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};

pub fn find_migrations_dir() -> Result<PathBuf> {
    let mut current_dir = env::current_exe()?
        .parent()
        .ok_or_else(|| color_eyre::eyre::eyre!("Executable has no parent directory"))?
        .to_path_buf();

    loop {
        let migrations_path = current_dir.join("migrations");
        if migrations_path.is_dir() {
            return Ok(migrations_path);
        }

        // Go up to the parent directory. If we are at the root, stop.
        if !current_dir.pop() {
            return Err(color_eyre::eyre::eyre!(
                "Could not find the 'migrations' directory in any parent path"
            ));
        }
    }
}

/// Run migrations and get a database connection pool.
/// # Errors
///
/// * `env::var` can return an error if `DATABASE_URL` is not found in the environment.
/// * `PgPool::connect` can return an error if the database connection fails.
/// * `sqlx::migrate` can return an error if migrations fail.
pub async fn get_db_pool(database_url: &str, run_migrations: bool) -> Result<Pool<Postgres>> {
    info!(
        "Connecting to database: {}",
        database_url
            .split('/')
            .next_back()
            .unwrap_or("DB NOT FOUND")
    );
    let db_config = &constants().database;
    let pool = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .min_connections(db_config.min_connection)
        .max_lifetime(Duration::from_secs(db_config.max_lifetime))
        .idle_timeout(Duration::from_secs(db_config.idle_timeout))
        .acquire_timeout(Duration::from_secs(db_config.acquire_timeout))
        .test_before_acquire(true)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET plan_cache_mode = force_custom_plan")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    if run_migrations {
        let migrations_folder = find_migrations_dir()?;
        let migrator = Migrator::new(migrations_folder).await?;
        match migrator.run(&pool).await {
            Ok(()) => info!("Database migrations completed successfully."),
            Err(e) => warn!("Database doesn't feel like migrating today: {e:?}"),
        }
    }
    Ok(pool)
}

#[must_use]
pub fn with_fallback_timezone(
    utc_dt: Option<DateTime<Utc>>,
    local_dt: &NaiveDateTime,
) -> DateTime<Utc> {
    utc_dt.unwrap_or_else(|| {
        constants().fallback_timezone.as_ref().map_or_else(
            || local_dt.and_utc(),
            |tz| {
                tz.from_local_datetime(local_dt)
                    .earliest()
                    .expect("Can't get datetime at timezone.")
                    .with_timezone(&Utc)
            },
        )
    })
}
