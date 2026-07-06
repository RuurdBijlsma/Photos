import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import type { IngestOverviewResponse } from '@/scripts/types/api/ingestJobs.ts'
import type { JobInfo } from '@/scripts/types/api/admin.ts'

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
   * Get failed ingest jobs for the current user.
   */
  getFailed(): Promise<AxiosResponse<JobInfo[]>> {
    return apiClient.get<JobInfo[]>('/jobs/ingest/failed')
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
