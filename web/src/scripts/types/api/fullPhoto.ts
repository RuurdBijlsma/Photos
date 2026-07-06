import type { Theme } from '@/scripts/types/themeColor.ts'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

export interface QualityScore {
  exposure: number
  contrast: number
  sharpness: number
  color_accuracy: number
  composition: number
  subject_clarity: number
  visual_impact: number
  creativity: number
  color_harmony: number
  storytelling: number
  style_suitability: number
  weighted_score: number
  measured_blurriness: number
  measured_noisiness: number
  measured_exposure: number
  measured_weighted_score: number
}

export interface ClassificationData {
  caption: string
  main_subject: string
  setting: string
  search_term: string
  contains_pets: boolean
  contains_vehicle: boolean
  contains_landmarks: boolean
  contains_people: boolean
  contains_animals: boolean
  contains_text: boolean
  is_indoor: boolean
  is_food: boolean
  is_drink: boolean
  is_event: boolean
  is_document: boolean
  is_landscape: boolean
  is_cityscape: boolean
  is_activity: boolean
  ocr_text?: string
  animal_type?: string
  food_name?: string
  drink_name?: string
  vehicle_type?: string
  event_type?: string
  landmark_name?: string
  document_type?: string
  people_count?: number
  people_mood?: string
  photo_type?: string
  activity_description?: string
}

export interface Face {
  position_x: number
  position_y: number
  width: number
  height: number
  confidence: number
  age: number
  sex: string
}

export interface ColorData {
  themes: Theme[]
  prominent_colors?: string[]
  average_hue: number
  average_saturation: number
  average_lightness: number
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  histogram?: { [key: string]: any }
}

export interface VisualAnalysis {
  created_at: string
  faces: Face[]
  colors?: ColorData
  quality?: QualityScore
  classification?: ClassificationData
}

export interface TimeDetails {
  timezone_source?: string
  source_details: string
  source_confidence: string
}

export interface MediaFeatures {
  mime_type: string
  size_bytes: number
  is_motion_photo: boolean
  motion_photo_presentation_timestamp?: number
  is_hdr: boolean
  is_burst: boolean
  burst_id?: string
  capture_fps?: number
  video_fps?: number
  is_nightsight: boolean
  is_timelapse: boolean
  exif: { [key: string]: unknown }
  audio_format?: string
  audio_channels?: number
  audio_sample_rate?: number
  compressor_id?: string
}

export interface CameraSettings {
  iso?: number
  exposure_time?: number
  aperture?: number
  focal_length?: number
  focal_length_in_35mm?: number
  camera_make?: string
  camera_model?: string
  digital_zoom_ratio?: number
  flash_fired?: boolean
  flash_mode?: string
  lens_make?: string
  lens_model?: string
  exposure_compensation?: number
}

export interface Location {
  name: string | null
  admin1: string | null
  admin2: string | null
  country_code: string
  country_name: string
}

export interface Gps {
  latitude: number
  longitude: number
  altitude?: number
  image_direction?: number
  location?: Location
}

export interface Weather {
  temperature?: number
  dew_point?: number
  relative_humidity?: number
  precipitation?: number
  snow?: number
  wind_direction?: number
  wind_speed?: number
  peak_wind_gust?: number
  pressure?: number
  sunshine_minutes?: number
  condition?: string
  sunrise?: string
  sunset?: string
  dawn?: string
  dusk?: string
  is_daytime?: boolean
}

export interface MediaItemAlbumRef {
  id: string
  name: string
  thumbnail_id: string | null
  media_count: number
}

export interface MediaItemWithAlbums {
  media_item: FullMediaItem
  albums: MediaItemAlbumRef[]
}

export interface FullMediaItem {
  id: string
  user_id: number
  hash: string
  filename: string
  relative_path: string
  created_at: string
  updated_at: string
  width: number
  height: number
  is_video: boolean
  duration_ms?: number
  taken_at_local: string
  taken_at_utc?: string
  og_taken_at_local: string
  timezone_name?: string
  timezone_offset_seconds?: number
  og_timezone_offset_seconds?: number
  use_panorama_viewer: boolean
  panorama_config: PannellumConfig
  has_thumbnails: boolean
  visual_analyses: VisualAnalysis[]
  gps?: Gps
  time: TimeDetails
  weather?: Weather
  media_features: MediaFeatures
  camera_settings: CameraSettings
  user_caption?: string
}
