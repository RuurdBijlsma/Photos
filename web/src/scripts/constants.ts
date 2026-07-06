export const DAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday']

export const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
]

export const CURRENT_YEAR = new Date().getFullYear()

export const THUMBNAIL_SIZES = [144, 240, 360, 480, 720, 1080, 1440]
export const VIDEO_SIZES = [144, 480]

export const WEATHER_ICONS: Record<string, string | { day: string; night: string }> = {
  Clear: {
    day: 'clear_day.svg',
    night: 'clear_night.svg',
  },

  Fair: {
    day: 'partly_cloudy_day.svg',
    night: 'partly_cloudy_night.svg',
  },

  Cloudy: 'cloudy_day_night.svg',
  Overcast: 'cloudy_day_night.svg',

  Fog: 'fog_day_night.svg',
  'Freezing Fog': 'fog_day_night.svg',

  'Light Rain': 'drizzle_day_night.svg',
  Rain: 'rain_day_night.svg',
  'Heavy Rain': 'rain_day_night.svg',

  'Freezing Rain': 'sleet_day_night.svg',
  'Heavy Freezing Rain': 'sleet_day_night.svg',

  Sleet: 'sleet_day_night.svg',
  'Heavy Sleet': 'sleet_day_night.svg',

  'Light Snowfall': 'snow_day_night.svg',
  Snowfall: 'snow_day_night.svg',
  'Heavy Snowfall': 'snow_day_night.svg',

  'Rain Shower': 'rain_day_night.svg',
  'Heavy Rain Shower': 'rain_day_night.svg',

  'Sleet Shower': 'sleet_day_night.svg',
  'Heavy Sleet Shower': 'sleet_day_night.svg',

  'Snow Shower': 'snow_day_night.svg',
  'Heavy Snow Shower': 'snow_day_night.svg',

  Lightning: 'thunder_day_night.svg',

  Hail: 'hail_day_night.svg',

  Thunderstorm: 'thunderstorm_day_night.svg',
  'Heavy Thunderstorm': 'thunderstorm_day_night.svg',

  Storm: 'wind_day_night.svg',
}

export const themeOptions = ['light', 'dark', 'system', 'schedule']
export type ThemeType = 'light' | 'dark' | 'system' | 'schedule'

export const themeVariantOptions = [
  /// grayscale
  'Monochrome',
  /// near-neutral palette
  'Neutral',
  /// calm, sedated colors
  'TonalSpot',
  ///  highly saturated
  'Vibrant',
  /// highly colorful, playful
  'Expressive',
  /// maximally faithful to source color
  'Fidelity',
  /// colors derived closely from the source color
  'Content',
  /// rainbow-like palette
  'Rainbow',
  /// multiple hues
  'FruitSalad',
  'Cmf',
]
export type ThemeVariant =
  | 'Monochrome'
  | 'Neutral'
  | 'TonalSpot'
  | 'Vibrant'
  | 'Expressive'
  | 'Fidelity'
  | 'Content'
  | 'Rainbow'
  | 'FruitSalad'
  | 'Cmf'
