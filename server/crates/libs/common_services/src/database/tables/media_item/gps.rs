use crate::database::media_item::location::Location;
use media_analyzer::GpsInfo;
use serde::{Deserialize, Serialize};

/// A composite struct representing data from the 'gps' table, with its associated 'location' data nested inside.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Gps {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub compass_direction: Option<f64>,
    pub location: Location,
}

/// Converts from the analysis result's `GpsInfo` to the database model `Gps`.
impl From<GpsInfo> for Gps {
    fn from(gps_info: GpsInfo) -> Self {
        Self {
            latitude: gps_info.latitude,
            longitude: gps_info.longitude,
            altitude: gps_info.altitude,
            compass_direction: gps_info.image_direction,
            location: gps_info.location.into(),
        }
    }
}
