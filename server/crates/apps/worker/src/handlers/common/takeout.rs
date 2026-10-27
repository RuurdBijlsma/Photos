use chrono::{DateTime, Utc};
use color_eyre::Result;
use common_services::caching::hash_file;
use common_services::database::media_item::media_item::CreateFullMediaItem;
use common_services::utils::write_media_datetime;
use media_analyzer::MediaMetadata;
use std::path::Path;
use tracing::warn;

/// Applies a Takeout sidecar timestamp to the DB fields.
pub fn apply_to_media_item(media_item: &mut CreateFullMediaItem, taken_at: DateTime<Utc>) {
    media_item.taken_at_local = taken_at.naive_utc();
    media_item.taken_at_utc = Some(taken_at);
    media_item.og_taken_at_local = media_item.taken_at_local;

    // TODO: sidecar timestamps are UTC; derive the real photo timezone and local time.
    media_item.timezone_name = Some("UTC".to_string());
    media_item.timezone_offset_seconds = Some(0);
    media_item.og_timezone_offset_seconds = Some(0);
    media_item.time.timezone_source = Some("Google Takeout sidecar (UTC)".to_string());
    media_item.time.source_details = "Google Takeout sidecar".to_string();
    media_item.time.source_confidence = "Medium".to_string();
}

/// Resolves a sidecar timestamp and writes it to the file when the analyzer is low-confidence.
pub async fn resolve_override(
    media_info: &mut MediaMetadata,
    file_path: &Path,
    fallback_taken_at: Option<DateTime<Utc>>,
) -> Result<Option<DateTime<Utc>>> {
    let Some(taken_at) = low_confidence_override(
        &media_info.time.source_details.confidence,
        fallback_taken_at,
    ) else {
        return Ok(None);
    };

    let formatted = taken_at.format("%Y:%m:%d %H:%M:%S").to_string();
    let target_path = file_path.to_path_buf();
    let is_video = media_info.features.is_video;
    let write_result = tokio::task::spawn_blocking(move || {
        write_media_datetime(&target_path, &formatted, is_video)
    })
    .await?;

    match write_result {
        Ok(()) => media_info.hash = hash_file(file_path)?,
        Err(e) => warn!(
            "Failed to write sidecar datetime to {:?}: {}; the DB override will still be applied.",
            file_path, e
        ),
    }

    Ok(Some(taken_at))
}

const fn low_confidence_override(
    confidence: &str,
    fallback_taken_at: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    if confidence.eq_ignore_ascii_case("low") {
        fallback_taken_at
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::low_confidence_override;
    use chrono::{DateTime, Utc};

    #[test]
    fn override_only_applies_to_low_confidence() {
        let dt = DateTime::<Utc>::from_timestamp(1_690_000_000, 0).expect("valid timestamp");

        assert_eq!(low_confidence_override("low", Some(dt)), Some(dt));
        assert_eq!(low_confidence_override("Medium", Some(dt)), None);
        assert_eq!(low_confidence_override("High", Some(dt)), None);
        assert_eq!(low_confidence_override("low", None), None);
    }
}
