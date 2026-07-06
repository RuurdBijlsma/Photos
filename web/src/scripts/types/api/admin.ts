// Maps to the PathInfoResponse schema
export interface PathInfoResponse {
  folder: string
  diskAvailable: number
  diskUsed: number
  diskTotal: number
  readAccess: boolean
  writeAccess: boolean
}

// Maps to the DiskResponse schema
export interface DiskResponse {
  mediaFolder: PathInfoResponse
  appDataFolder: PathInfoResponse
}

// Maps to the MakeFolderBody schema
export interface MakeFolderBody {
  baseFolder: string
  newName: string
}

// Maps to the MakeFolderBody schema
export interface StartProcessingBody {
  userFolder: string
}

// Maps to the MediaSampleResponse schema
export interface MediaSampleResponse {
  readAccess: boolean
  folder: string
  photoCount: number
  videoCount: number
  samples: string[]
}

// Maps to the UnsupportedFilesResponse schema
export interface UnsupportedFilesResponse {
  readAccess: boolean
  folder: string
  inaccessibleEntries: string[]
  unsupportedFiles: Record<string, string[]> // Represents a dictionary of string arrays
  unsupportedCount: number
}

// --- Admin-specific types ---

export interface AdminUserInfo {
  id: number
  username: string
  email: string
  avatarId: string | null
  mediaFolder: string | null
  mainDriveUsed: number
}

// jobs

export type JobType =
  | 'ingest_metadata'
  | 'ingest_thumbnails'
  | 'ingest_analysis'
  | 'ingest_llm'
  | 'remove'
  | 'scan'
  | 'clean_db'
  | 'cluster_faces'
  | 'cluster_photos'
  | 'import_album_item'
  | 'update_global_centroid'
  | 'sync_thumbnails'
  | 'delayed_scan'
  | 'generate_daily_cards'
  | 'calc_system_stats'

export type JobStatus = 'queued' | 'running' | 'failed' | 'done' | 'cancelled'

export interface JobInfo {
  id: number
  relativePath: string | null
  userId: number | null
  jobType: JobType
  payload: unknown | null
  priority: number
  status: JobStatus
  attempts: number
  dependencyAttempts: number
  maxAttempts: number
  owner: string | null
  startedAt: string | null
  finishedAt: string | null
  createdAt: string
  scheduledAt: string
  lastHeartbeat: string
  lastError: string | null
}

export interface PaginatedJobsResponse {
  data: JobInfo[]
  total: number
  limit: number
  offset: number
}
