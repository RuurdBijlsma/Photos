export interface IngestJobCounts {
  queued: number
  running: number
  failed: number
  done: number
  cancelled: number
  total: number
}

export interface IngestOverviewResponse {
  metadata: IngestJobCounts
  thumbnails: IngestJobCounts
  analysis: IngestJobCounts
  mediaFolder?: {
    available: boolean
    reason?: string
  }
}

export interface RetryJobPayload {
  id: number
}
