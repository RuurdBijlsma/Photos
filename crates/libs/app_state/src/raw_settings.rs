use material_color_utils::dynamic::variant::Variant;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct RawSettings {
    /// Folder with source photos and video
    pub ingest: RawIngestSettings,
    pub logging: LoggingSettings,
    pub api: ApiSettings,
    pub secrets: SecretSettings,
    pub constants: RawConstants,
    pub daily_cards: DailyCardsSettings,
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
    pub llm_base_url: String,
    pub theme_generation: ThemeSettings,
    pub search: SearchSettings,
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
#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    pub level: String,
}

/// Configuration for the API server.
#[derive(Debug, Deserialize, Clone)]
pub struct ApiSettings {
    pub host: String,
    pub port: u32,
    pub allowed_origins: Vec<String>,
    pub public_url: String,
    pub rate_limiting: RateLimitingSettings,
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DailyCardsSettings {
    pub cluster: CardLimits,
    pub estimatr: EstimatrSettings,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CardLimits {
    pub min_cards: usize,
    pub max_cards: usize,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EstimatrSettings {
    pub rounds_per_day: i64,
}
