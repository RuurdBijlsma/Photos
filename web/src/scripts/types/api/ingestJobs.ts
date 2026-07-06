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
  llm: IngestJobCounts
}

export interface RetryJobPayload {
  id: number
}
