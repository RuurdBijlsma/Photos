use media_analyzer::TimeInfo;
use serde::{Deserialize, Serialize};

/// Corresponds to the '`time_details`' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeDetails {
    pub timezone_source: Option<String>,
    pub source_details: String,
    pub source_confidence: String,
}

/// Converts from the analysis result's `TimeInfo` to the database model `TimeDetails`.
impl From<TimeInfo> for TimeDetails {
    fn from(time_info: TimeInfo) -> Self {
        Self {
            timezone_source: time_info.timezone.map(|tz| tz.source),
            source_details: time_info.source_details.time_source,
            source_confidence: time_info.source_details.confidence,
        }
    }
}
