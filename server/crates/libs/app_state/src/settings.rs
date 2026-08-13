use crate::constants::{
    CACHE_FOLDER, FACE_CLUSTERS_FOLDER, HUGGINGFACE_CACHE_FOLDER, ON_DEMAND_THUMBNAIL_CACHE_FOLDER,
    PANO_FOLDER, THUMBNAILS_FOLDER,
};
use crate::{
    AnalyzerSettings, ApiSettings, DailyCardsSettings, FileDetectionSettings, LoggingSettings,
    MakeRelativePath, RawSettings, SecretSettings, ThumbnailSettings,
};
use color_eyre::Result;
use serde::Deserialize;
use sqlx::{Executor, Postgres};
use std::fs::canonicalize;
use std::path::{Path, PathBuf, absolute};
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct AppSettings {
    pub ingest: IngestSettings,
    pub logging: LoggingSettings,
    pub api: ApiSettings,
    pub secrets: SecretSettings,
    pub daily_cards: DailyCardsSettings,
    pub scaler: ScalerSettings,
}

/// Defines paths for media and thumbnail storage.
#[derive(Debug, Deserialize, Clone)]
pub struct IngestSettings {
    pub media_root: PathBuf,
    pub media_root_canon: PathBuf,
    pub app_data_root: PathBuf,
    pub thumbnails_root: PathBuf,
    pub on_demand_thumbs_cache_root: PathBuf,
    pub cache_root: PathBuf,
    pub pano_root: PathBuf,
    pub hf_cache_root: PathBuf,
    pub face_clusters_root: PathBuf,
    pub enable_cache: bool,
    pub analyzer: AnalyzerSettings,
    pub file_detection: FileDetectionSettings,
    pub thumbnails: ThumbnailSettings,
}

impl From<RawSettings> for AppSettings {
    fn from(raw: RawSettings) -> Self {
        let app_data_root =
            absolute(&raw.ingest.app_data_folder).expect("Invalid thumbnail_folder");
        let media_root = absolute(&raw.ingest.media_folder).expect("Invalid media_folder");
        let media_root_canon =
            canonicalize(&raw.ingest.media_folder).expect("Invalid media_folder");
        let on_demand_thumbs_cache_root = app_data_root.join(ON_DEMAND_THUMBNAIL_CACHE_FOLDER);
        let pano_root = app_data_root.join(PANO_FOLDER);
        let cache_root = app_data_root.join(CACHE_FOLDER);
        let face_clusters_root = app_data_root.join(FACE_CLUSTERS_FOLDER);
        let thumbnails_root = app_data_root.join(THUMBNAILS_FOLDER);
        let hf_cache_root = app_data_root.join(HUGGINGFACE_CACHE_FOLDER);
        let ingest = IngestSettings {
            media_root_canon,
            media_root,
            app_data_root,
            enable_cache: raw.ingest.enable_cache,
            analyzer: raw.ingest.analyzer,
            file_detection: raw.ingest.file_detection,
            thumbnails: raw.ingest.thumbnails,
            on_demand_thumbs_cache_root,
            pano_root,
            hf_cache_root,
            face_clusters_root,
            thumbnails_root,
            cache_root,
        };

        let mut scaler_profiles: Vec<ProfileSettings> = raw
            .scaler
            .profiles
            .into_iter()
            .map(|(k, v)| ProfileSettings {
                name: k,
                priority: v.priority,
                estimated_ram_mb: v.estimated_ram_mb,
                max_workers: v.max_workers,
                excluded_jobs: v.excluded_jobs,
            })
            .collect();
        scaler_profiles.sort_by_key(|b| std::cmp::Reverse(b.priority));

        let scaler = ScalerSettings {
            tick_interval: Duration::from_secs(raw.scaler.tick_interval_secs),
            quick_tick_interval: Duration::from_secs(raw.scaler.quick_tick_interval_secs),
            system_memory_buffer_percentage: raw.scaler.system_memory_buffer_percentage,
            system_memory_buffer_maximum_mb: raw.scaler.system_memory_buffer_maximum_mb,
            unload_models_timeout: Duration::from_secs(raw.scaler.unload_models_timeout_secs),
            profiles: scaler_profiles,
        };

        Self {
            ingest,
            logging: raw.logging,
            api: raw.api,
            secrets: raw.secrets,
            daily_cards: raw.daily_cards,
            scaler,
        }
    }
}

impl IngestSettings {
    // stuff that needs multiple settings (otherwise just make it a standalone function).

    #[must_use]
    pub fn is_media_file(&self, file: &Path) -> bool {
        let Some(extension) = file.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
            return false;
        };
        let is_photo = self.file_detection.photo_extensions.contains(&extension);
        let is_video = self.file_detection.video_extensions.contains(&extension);
        is_photo || is_video
    }

    #[must_use]
    pub fn is_photo_file(&self, file: &Path) -> bool {
        let Some(extension) = file.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
            return false;
        };
        self.file_detection.photo_extensions.contains(&extension)
    }

    #[must_use]
    pub fn is_video_file(&self, file: &Path) -> bool {
        let Some(extension) = file.extension().map(|e| e.to_string_lossy().to_lowercase()) else {
            return false;
        };
        self.file_detection.video_extensions.contains(&extension)
    }

    /// Verifies if all expected thumbnails for a given media file exist on disk.
    /// # Errors
    ///
    /// This function's signature returns a Result, but the current implementation does not produce any errors.
    pub fn thumbs_exist(
        &self,
        file: &Path,
        thumb_sub_folder: &str,
        required_pano_folder: Option<&Path>,
        require_motion_photo: bool,
    ) -> Result<bool> {
        let is_photo = self.is_photo_file(file);
        let is_video = self.is_video_file(file);
        let photo_thumb_ext = &self.thumbnails.thumbnail_extension;
        let video_thumb_ext = &self.thumbnails.video_options.extension;
        let mut should_exist: Vec<String> = vec![];

        if is_photo || is_video {
            // Both photo and video should have a thumbnail for each entry in .heights.
            for h in &self.thumbnails.heights {
                should_exist.push(format!("{h}p.{photo_thumb_ext}"));
            }
        }
        if is_video {
            for p in &self.thumbnails.video_options.percentages {
                should_exist.push(format!("{p}_percent.{photo_thumb_ext}"));
            }
            for x in &self.thumbnails.video_options.transcode_outputs {
                let height = x.height;
                should_exist.push(format!("{height}p.{video_thumb_ext}"));
            }
        }

        let thumb_dir = self.thumbnails_root.join(thumb_sub_folder);
        for thumb_filename in should_exist {
            let thumb_file_path = thumb_dir.join(thumb_filename.clone());
            if !thumb_file_path.exists() {
                return Ok(false);
            }
        }

        if is_photo {
            if require_motion_photo {
                let motion_file = thumb_dir.join("motion.mp4");
                if !motion_file.exists() {
                    return Ok(false);
                }
            }

            if let Some(pano_folder) = required_pano_folder {
                let config_file = pano_folder.join("config.json");
                if !config_file.exists() {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Checks if a file has already been ingested by verifying its database record and thumbnail existence.
    /// # Errors
    ///
    /// * Can return an error from `thumbs_exist` if checking for thumbnails fails.
    pub async fn file_is_ingested(
        &self,
        executor: impl Executor<'_, Database = Postgres>,
        file: &Path,
    ) -> Result<bool> {
        // Media item existence check:
        let Ok(relative_path_str) = file.make_relative(&self.media_root) else {
            return Ok(false);
        };
        let Ok(media_item_id) = sqlx::query_scalar!(
            "SELECT id FROM media_item WHERE relative_path = $1",
            relative_path_str
        )
        .fetch_optional(executor)
        .await
        else {
            return Ok(false);
        };
        let Some(media_item_id) = media_item_id else {
            return Ok(false);
        };
        // media item exists, check thumbnails existence
        let exist = self.thumbs_exist(file, &media_item_id, None, false)?;
        Ok(exist)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScalerSettings {
    pub tick_interval: Duration,
    pub quick_tick_interval: Duration,
    pub system_memory_buffer_percentage: f64,
    pub system_memory_buffer_maximum_mb: u64,
    pub unload_models_timeout: Duration,
    pub profiles: Vec<ProfileSettings>, // sorted by priority, descending
}

#[derive(Debug, Clone, Deserialize, Hash, PartialEq, Eq)]
pub struct ProfileSettings {
    pub name: String,
    pub priority: i32,
    pub estimated_ram_mb: u64,
    pub max_workers: usize,
    pub excluded_jobs: Vec<String>,
}
