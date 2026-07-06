use app_state::constants::{
    FACE_CLUSTERS_FOLDER, ON_DEMAND_THUMBNAIL_CACHE_FOLDER, PANO_FOLDER, THUMBNAILS_FOLDER,
};
use app_state::{AppSettings, database_url};
use color_eyre::eyre::Result;
use common_services::database::get_db_pool;
use sqlx::{Executor, PgPool};
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use tempfile::TempDir;
use tracing::info;
use url::Url;

pub fn create_test_settings(
    database_name: &str,
    base_settings: &AppSettings,
) -> Result<(AppSettings, TempDir, TempDir, String)> {
    // 1. Load base settings from the test configuration file.
    let mut settings = base_settings.clone();

    // 2. Create temporary directories for media and thumbnails.
    let media_dir = TempDir::new()?;
    let app_data_dir = TempDir::new()?;
    let app_data_path = app_data_dir.path();
    let port = get_free_port();
    settings.api.port = u32::from(port);
    settings.api.public_url = format!("http://127.0.0.1:{port}");
    settings.ingest.media_root = media_dir.path().to_path_buf();
    settings.ingest.media_root_canon = media_dir.path().canonicalize()?;
    settings.ingest.app_data_root = app_data_path.to_path_buf();
    settings.ingest.face_clusters_root = app_data_path.join(FACE_CLUSTERS_FOLDER);
    settings.ingest.pano_root = app_data_path.join(PANO_FOLDER);
    settings.ingest.on_demand_thumbs_cache_root =
        app_data_path.join(ON_DEMAND_THUMBNAIL_CACHE_FOLDER);
    settings.ingest.thumbnails_root = app_data_path.join(THUMBNAILS_FOLDER);

    // 3. Update the database URL to point to our unique test database.
    let mut db_url = Url::parse(database_url())?;
    db_url.set_path(&format!("/{database_name}"));

    Ok((settings, media_dir, app_data_dir, db_url.to_string()))
}

pub async fn create_test_database(
    base_database_url: &str,
    database_name: &str,
) -> Result<(PgPool, PgPool)> {
    // 1. Connect to the default 'postgres' database to manage other databases.
    let mut management_db_url = Url::parse(base_database_url)?;
    management_db_url.set_path("/postgres");
    let management_pool = get_db_pool(management_db_url.as_str(), false).await?;
    force_drop_db(&management_pool, database_name)
        .await
        .expect("Failed to clean up DB.");

    // 2. Create the new test database.
    management_pool
        .execute(format!("CREATE DATABASE \"{database_name}\"").as_str())
        .await?;

    // 3. Connect to the newly created test database.
    let mut test_db_url = Url::parse(base_database_url)?;
    test_db_url.set_path(&format!("/{database_name}"));
    let main_pool = get_db_pool(test_db_url.as_str(), true).await?;

    // 4. Run migrations on the test database.
    sqlx::migrate!("../../migrations").run(&main_pool).await?;
    info!("Finished database migrations for {}", database_name);

    Ok((main_pool, management_pool))
}

pub async fn force_drop_db(management_pool: &PgPool, db_name: &str) -> Result<()> {
    let _ = management_pool
        .execute(format!("DROP DATABASE \"{db_name}\" WITH (FORCE)").as_str())
        .await;
    Ok(())
}

pub fn copy_dir_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.as_ref().join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(entry.path(), dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

#[must_use]
pub fn get_free_port() -> u16 {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Can't bind tcp listener on 120.0.0.1:0");
    listener
        .local_addr()
        .expect("Can't get local address from listener.")
        .port()
}
