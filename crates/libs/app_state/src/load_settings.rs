use crate::{AppConstants, AppSettings, IngestSettings, RawSettings};
use color_eyre::eyre::Result;
use config::{Config, File};
use std::path::Path;
use std::sync::OnceLock;
use std::{env, fs};

pub fn load_settings_from_path(path: &Path, env_path: Option<&Path>) -> Result<AppSettings> {
    // Need to load from dotenv to get it to overwrite the secrets from env.
    if let Some(env_path) = env_path {
        dotenv::from_path(env_path).ok();
    }

    let builder = {
        let mut builder = Config::builder().add_source(File::from(path));
        if env_path.is_some() {
            builder = builder.add_source(
                config::Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true),
            );
        }
        builder
    };

    let raw_settings = builder.build()?.try_deserialize::<RawSettings>()?;
    let settings: AppSettings = raw_settings.into();
    init_app_data_folders(&settings.ingest);

    Ok(settings)
}

pub fn init_app_data_folders(settings: &IngestSettings) {
    fs::create_dir_all(&settings.app_data_root).expect("Cannot create app_data_root folder");
    fs::create_dir_all(&settings.thumbnails_root).expect("Cannot create thumbnails_root folder");
    fs::create_dir_all(&settings.pano_root).expect("Cannot create pano_root folder");
    fs::create_dir_all(&settings.cache_root).expect("Cannot create cache_root folder");
    fs::create_dir_all(&settings.on_demand_thumbs_cache_root)
        .expect("Cannot create on_demand_thumbs_cache_root folder");
    fs::create_dir_all(&settings.face_clusters_root)
        .expect("Cannot create face_clusters_root folder");
}

pub fn load_constants_from_path(path: &Path) -> Result<AppConstants> {
    let builder = Config::builder().add_source(File::from(path));
    let raw_constants = builder.build()?.try_deserialize::<RawSettings>()?;
    let app_constants: AppConstants = raw_constants.into();

    Ok(app_constants)
}

pub fn load_app_settings() -> Result<AppSettings> {
    let config_path = Path::new("config/settings.yaml").canonicalize()?;
    load_settings_from_path(&config_path, Some(Path::new(".env")))
}

fn load_app_constants() -> Result<AppConstants> {
    let config_path = Path::new("config/settings.yaml").canonicalize()?;
    load_constants_from_path(&config_path)
}

pub static CONSTANTS: OnceLock<AppConstants> = OnceLock::new();

#[must_use]
pub fn constants() -> &'static AppConstants {
    CONSTANTS.get_or_init(|| load_app_constants().expect("Cannot load app settings."))
}

pub static DATABASE_URL: OnceLock<String> = OnceLock::new();

#[must_use]
pub fn database_url() -> &'static String {
    DATABASE_URL.get_or_init(|| env::var("DATABASE_URL").expect("No DATABASE_URL env variable set"))
}
