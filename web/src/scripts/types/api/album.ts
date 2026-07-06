import type { TimelineItem } from '@/scripts/types/generated/timeline'
import type {
  CameraSettings,
  TimeDetails,
  VisualAnalysis,
  Weather,
} from '@/scripts/types/api/fullPhoto.ts'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

// Enums
export type AlbumRole = 'Owner' | 'Contributor' | 'Viewer'
export type SortDirection = 'asc' | 'desc'
export type AlbumSortField = 'updatedAt' | 'latestPhoto' | 'name'
export type AlbumSort = 'DateAsc' | 'DateDesc' | 'AddedAsc' | 'AddedDesc' | 'None'

// Entities
export interface Album {
  id: string
  ownerId: number
  name: string
  description: string | null
  thumbnailId: string | null
  isPublic: boolean
  createdAt: string // ISO Date string
  updatedAt: string // ISO Date string
  mediaCount: number
  latestMediaItemTimestamp: string | null // ISO Date string
  earliestMediaItemTimestamp: string | null // ISO Date string
  sortMode: AlbumSort
}

export interface AlbumCollaborator {
  id: number
  albumId: string
  userId: number
  role: AlbumRole
  addedAt: string // ISO Date string
}

export interface AlbumMediaItemSummary {
  mediaItem: TimelineItem
  addedAt: string // ISO Date string
}

export interface AlbumSummary {
  name: string
  description: string | null
  relativePaths: string[]
}

// Request Payloads
export interface CreateAlbumRequest {
  name: string
  description?: string
  isPublic: boolean
  mediaItemIds: string[]
}

export interface UpdateAlbumRequest {
  name?: string
  isPublic?: boolean
  description?: string | null
  thumbnailId?: string | null
}

export interface AddMediaToAlbumRequest {
  mediaItemIds: string[]
}

export interface AddCollaboratorRequest {
  userId: number
  role: AlbumRole
}

export interface CheckInviteRequest {
  token: string
}

export interface AcceptInviteRequest {
  token: string
  name: string
  description?: string
}

export interface SharedMediaFeatures {
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
  audio_format?: string
  audio_channels?: number
  audio_sample_rate?: number
  compressor_id?: string
}

export interface SharedMediaItem {
  id: string
  filename: string
  width: number
  height: number
  is_video: boolean
  duration_ms: number | null
  taken_at_local: string // NaiveDateTime
  taken_at_utc: string | null // DateTime<Utc>
  timezone_name: string | null
  timezone_offset_seconds: number | null
  panorama_config: PannellumConfig
  use_panorama_viewer: boolean
  has_thumbnails: boolean
  visual_analyses: VisualAnalysis[]
  time: TimeDetails
  weather: Weather | null
  media_features: SharedMediaFeatures
  camera_settings: CameraSettings
  user_caption: string | null
  gps: null
}

export interface BackupInfo {
  filename: string
  sizeBytes: number
  createdAt: string // ISO Date string
}
