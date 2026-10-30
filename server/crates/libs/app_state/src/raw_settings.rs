use material_color_utils::dynamic::variant::Variant;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct RawSettings {
    pub ingest: RawIngestSettings,
    pub logging: LoggingSettings,
    pub api: ApiSettings,
    pub secrets: SecretSettings,
    pub constants: RawConstants,
    pub daily_cards: DailyCardsSettings,
    pub scaler: RawScalerSettings,
}

/// Defines paths for media and thumbnail storage.
#[derive(Debug, Deserialize, Clone)]
pub struct RawIngestSettings {
    pub media_folder: PathBuf,
    pub app_data_folder: PathBuf,
    pub enable_cache: bool,
    pub analyzer: AnalyzerSettings,
    pub file_detection: FileDetectionSettings,
    pub thumbnails: ThumbnailSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyzerSettings {
    pub analyze_image_size: u64,
    pub theme_generation: ThemeSettings,
    pub search: SearchSettings,
    pub onnx: OnnxSettings,
    pub clustering: ClusteringSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct ClusteringSettings {
    pub session_time_gap_seconds: u64,
    pub session_distance_meters: f64,
    pub min_cluster_size: usize,
    pub min_samples: usize,
    pub card_max_photos: usize,
    pub card_min_photos: usize,
    pub mmr_relevance_lambda: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OnnxSettings {
    pub intra_threads: usize,
    pub inter_threads: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchSettings {
    pub embedder_model_id: String,
    pub semantic_score_threshold: f64,
    pub semantic_weight: f64,
    pub text_weight: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeSettings {
    pub variant: Variant,
}

/// Configuration for thumbnail generation settings.
#[derive(Debug, Deserialize, Clone)]
pub struct FileDetectionSettings {
    /// Which extensions are categorized as videos
    pub video_extensions: Vec<String>,
    /// Which extensions are categorized as photos
    pub photo_extensions: Vec<String>,
}

/// Configuration for thumbnail generation settings.
#[derive(Debug, Deserialize, Clone)]
pub struct ThumbnailSettings {
    pub recreate_if_exists: bool,
    /// The file extension for photo thumbnails (e.g., "avif", "webp", "jpg").
    pub thumbnail_extension: String,
    /// A vector of heights for generating multiple thumbnails.
    /// - For videos, these are the heights for stills taken at `thumb_time`.
    /// - For images, these are the heights for the generated thumbnails.
    pub heights: Vec<u64>,
    pub avif_options: AvifOptions,
    pub video_options: VideoThumbOptions,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoOutputFormat {
    /// The height of the output video in pixels. The width will be scaled automatically to maintain aspect ratio.
    pub height: u64,
    /// The quality setting for the video encoding. For VP9, this is the CRF (Constant Rate Factor) value.
    pub quality: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvifOptions {
    /// Quality 1..=100. Panics if out of range.
    pub quality: f32,
    /// Quality for the alpha channel only. `1..=100`. Panics if out of range.
    pub alpha_quality: f32,
    /// - 1 = very slow, but max compression.
    /// - 10 = quick, but larger file sizes and lower quality.
    ///
    /// Panics if outside 1..=10.
    pub speed: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoThumbOptions {
    /// The specific time in seconds from the start of the video to generate multi-size stills from.
    pub thumb_time: f64,
    /// A vector of percentages of the video's total duration at which to capture still images.
    pub percentages: Vec<u64>,
    /// The height in pixels for the thumbnails generated based on the `percentages` field.
    pub height: u64,
    /// A list of video formats to generate as previews from the source video.
    pub transcode_outputs: Vec<VideoOutputFormat>,
    /// The file extension for video transcoding (e.g., "webm", "mp4").
    pub extension: String,
}

/// Logging configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingSettings {
    /// Console (stdout) sink filters.
    pub filters: LoggingFilterSettings,
    /// Machine-readable file sink. Defaulted so settings files written before it existed still load.
    #[serde(default)]
    pub file: FileLoggingSettings,
}

/// Configuration for the JSON file sink under `app_data/logs`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileLoggingSettings {
    pub enabled: bool,
    /// Own filter set: the console's debug targets would otherwise flood the archive.
    pub filters: LoggingFilterSettings,
    /// Number of daily files kept; the appender deletes the oldest on rotation.
    pub max_files: usize,
}

impl Default for FileLoggingSettings {
    fn default() -> Self {
        let targets = [
            ("ort", "warn"),
            ("hf_hub", "error"),
            ("xet_client", "warn"),
            ("xet_data", "warn"),
            ("xet_runtime", "warn"),
            ("xet", "warn"),
            ("open_clip_inference", "warn"),
            ("object_detector", "warn"),
            ("face_id", "warn"),
            ("sqlx", "warn"),
        ]
        .into_iter()
        .map(|(target, level)| (target.to_owned(), level.to_owned()))
        .collect();

        Self {
            enabled: true,
            filters: LoggingFilterSettings {
                main: "info".to_owned(),
                targets,
            },
            max_files: 7,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoggingFilterSettings {
    pub main: String,
    pub targets: HashMap<String, String>,
}

impl LoggingFilterSettings {
    /// Formats the logging filters into an `EnvFilter` compatible directive string (e.g., "info,api=debug,ort=warn").
    #[must_use]
    pub fn to_filter_string(&self) -> String {
        let mut parts = vec![self.main.clone()];
        for (target, level) in &self.targets {
            parts.push(format!("{target}={level}"));
        }
        parts.join(",")
    }
}

/// Configuration for the API server.
#[derive(Debug, Deserialize, Clone)]
pub struct ApiSettings {
    pub host: String,
    pub port: u32,
    pub allowed_origins: Vec<String>,
    pub public_url: String,
    pub cookie_secure: bool,
    pub rate_limiting: RateLimitingSettings,
    pub serve_static_assets: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitingSettings {
    pub req_per_second: u64,
    pub burst_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretSettings {
    pub jwt: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RawConstants {
    pub fallback_timezone: String,
    pub onboarding_n_media_samples: usize,
    pub database: DatabaseConstants,
    pub auth: AuthConstants,
    pub allow_file_modifications: bool,
    pub allow_file_deletion: bool,
}

/// Database connection and related configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConstants {
    pub max_connections: u32,
    pub min_connection: u32,
    pub max_lifetime: u64,
    pub idle_timeout: u64,
    pub acquire_timeout: u64,
    /// Length of generated `id` to use for media item in database.
    pub media_item_id_length: usize,
    pub album_id_length: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConstants {
    pub access_token_expiry_minutes: i64,
    pub refresh_token_expiry_days: i64,
    pub album_invitation_expiry_minutes: i64,
    pub upload_token_expiry_minutes: i64,
}

#[derive(Debug, Deserialize, Clone, Copy, Serialize)]
pub struct DatelessCardPolicy {
    pub chance: f64,
    pub min_cards: usize,
    pub max_cards: usize,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DailyCardsSettings {
    pub dateless: HashMap<String, DatelessCardPolicy>,
    pub estimatr: EstimatrSettings,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EstimatrSettings {
    pub rounds_per_day: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RawProfileSettings {
    pub priority: i32,
    pub estimated_ram_mb: u64,
    pub max_workers: usize,
    pub excluded_jobs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RawScalerSettings {
    pub tick_interval_secs: u64,
    pub quick_tick_interval_secs: u64,
    pub system_memory_buffer_percentage: f64,
    pub system_memory_buffer_maximum_mb: u64,
    pub unload_models_timeout_secs: u64,
    pub profiles: HashMap<String, RawProfileSettings>,
}

#[cfg(test)]
mod tests {
    use super::{FileLoggingSettings, LoggingSettings};

    fn parse_logging(yaml: &str) -> LoggingSettings {
        config::Config::builder()
            .add_source(config::File::from_str(yaml, config::FileFormat::Yaml))
            .build()
            .expect("test config builds")
            .try_deserialize()
            .expect("test logging settings deserialize")
    }

    #[test]
    fn logging_settings_default_file_block_when_absent() {
        let parsed = parse_logging("filters:\n  main: info\n  targets: {}\n");
        assert!(parsed.file.enabled);
        assert_eq!(parsed.file.max_files, 7);
        assert_eq!(parsed.file.filters.main, "info");
    }

    #[test]
    fn file_filter_string_includes_quieted_targets() {
        let filter_string = FileLoggingSettings::default().filters.to_filter_string();
        assert!(filter_string.starts_with("info,"));
        assert!(filter_string.contains("hf_hub=error"));
        assert!(filter_string.contains("ort=warn"));
    }
}
