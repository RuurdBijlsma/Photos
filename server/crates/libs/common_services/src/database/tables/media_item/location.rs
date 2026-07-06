use media_analyzer::LocationName;
use serde::{Deserialize, Serialize};

/// Corresponds to the 'location' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Location {
    pub name: String,
    pub admin1: String,
    pub admin2: String,
    pub country_code: String,
    pub country_name: String,
}

impl From<LocationName> for Location {
    fn from(loc: LocationName) -> Self {
        Self {
            name: loc.name,
            admin1: loc.admin1,
            admin2: loc.admin2,
            country_code: loc.country_code,
            country_name: loc.country_name.unwrap_or_else(|| "N/A".to_owned()),
        }
    }
}
