use crate::{AuthConstants, DatabaseConstants, RawSettings};
use chrono_tz::Tz;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConstants {
    pub fallback_timezone: Option<Tz>,
    pub onboarding_n_media_samples: usize,
    pub database: DatabaseConstants,
    pub auth: AuthConstants,
    pub allow_file_modifications: bool,
    pub allow_file_deletion: bool,
}

impl From<RawSettings> for AppConstants {
    fn from(raw: RawSettings) -> Self {
        let fallback_timezone = get_fallback_timezone(&raw.constants.fallback_timezone);

        Self {
            fallback_timezone,
            onboarding_n_media_samples: raw.constants.onboarding_n_media_samples,
            database: raw.constants.database,
            auth: raw.constants.auth,
            allow_file_modifications: raw.constants.allow_file_modifications,
            allow_file_deletion: raw.constants.allow_file_deletion,
        }
    }
}

fn get_fallback_timezone(tz_string: &str) -> Option<Tz> {
    if tz_string.is_empty() {
        return None;
    }
    let parsed_tz = tz_string
        .parse::<Tz>()
        .unwrap_or_else(|_| panic!("Invalid fallback timezone: {tz_string}"));

    Some(parsed_tz)
}
