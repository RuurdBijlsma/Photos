use chrono::{DateTime, Utc};
use media_analyzer::WeatherInfo;
use serde::{Deserialize, Serialize};

/// Corresponds to the 'weather' table.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Weather {
    pub temperature: Option<f32>,
    pub dew_point: Option<f32>,
    pub relative_humidity: Option<i32>,
    pub precipitation: Option<f32>,
    pub snow: Option<i32>,
    pub wind_direction: Option<i32>,
    pub wind_speed: Option<f32>,
    pub peak_wind_gust: Option<f32>,
    pub pressure: Option<f32>,
    pub sunshine_minutes: Option<i32>,
    pub condition: Option<String>,
    pub sunrise: Option<DateTime<Utc>>,
    pub sunset: Option<DateTime<Utc>>,
    pub dawn: Option<DateTime<Utc>>,
    pub dusk: Option<DateTime<Utc>>,
    pub is_daytime: Option<bool>,
}

/// Converts from the analysis result's `WeatherInfo` to the database model `Weather`.
impl From<WeatherInfo> for Weather {
    fn from(weather_info: WeatherInfo) -> Self {
        let hourly = weather_info.hourly;
        Self {
            temperature: hourly
                .as_ref()
                .and_then(|h| h.temperature.map(|t| t as f32)),
            dew_point: hourly
                .as_ref()
                .and_then(|h| h.dew_point.map(|dp| dp as f32)),
            relative_humidity: hourly.as_ref().and_then(|h| h.relative_humidity),
            precipitation: hourly
                .as_ref()
                .and_then(|h| h.precipitation.map(|p| p as f32)),
            snow: hourly.as_ref().and_then(|h| h.snow),
            wind_direction: hourly.as_ref().and_then(|h| h.wind_direction),
            wind_speed: hourly
                .as_ref()
                .and_then(|h| h.wind_speed.map(|ws| ws as f32)),
            peak_wind_gust: hourly
                .as_ref()
                .and_then(|h| h.peak_wind_gust.map(|pwg| pwg as f32)),
            pressure: hourly.as_ref().and_then(|h| h.pressure.map(|p| p as f32)),
            sunshine_minutes: hourly.as_ref().and_then(|h| h.sunshine_minutes),
            condition: hourly.and_then(|h| h.condition.map(|c| c.to_string())),
            sunrise: weather_info.sun_info.sunrise,
            sunset: weather_info.sun_info.sunset,
            dawn: weather_info.sun_info.dawn,
            dusk: weather_info.sun_info.dusk,
            is_daytime: Some(weather_info.sun_info.is_daytime),
        }
    }
}
