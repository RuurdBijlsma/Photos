import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import type { IngestOverviewResponse } from '@/scripts/types/api/ingestJobs.ts'
import type { JobInfo, PaginatedJobsResponse } from '@/scripts/types/api/admin.ts'

const ingestJobsService = {
  /**
   * Get total and completed counts for each user ingestion task type.
   */
  getOverview(): Promise<AxiosResponse<IngestOverviewResponse>> {
    return apiClient.get<IngestOverviewResponse>('/jobs/ingest/overview')
  },

  /**
   * Get active/running ingest jobs for the current user.
   */
  getRunning(): Promise<AxiosResponse<JobInfo[]>> {
    return apiClient.get<JobInfo[]>('/jobs/ingest/running')
  },

  /**
   * Get paginated ingest jobs for the current user.
   */
  getUserJobs(params: {
    page?: number
    limit?: number
    status?: string
    search?: string
  }): Promise<AxiosResponse<PaginatedJobsResponse>> {
    return apiClient.get<PaginatedJobsResponse>('/jobs/ingest', { params })
  },

  /**
   * Trigger a scanning process of the user's library folder.
   */
  scan(): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/jobs/ingest/scan')
  },

  /**
   * Retry a failed ingestion job.
   */
  retry(jobId: number): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/jobs/ingest/retry', { id: jobId })
  },
}

export default ingestJobsService
