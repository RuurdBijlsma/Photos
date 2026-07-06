import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import type {
  DiskResponse,
  MakeFolderBody,
  MediaSampleResponse,
  UnsupportedFilesResponse,
  AdminUserInfo,
  PaginatedJobsResponse,
} from '@/scripts/types/api/admin.ts'

const adminService = {
  /**
   * Get information about the configured media and thumbnail disks.
   */
  getDisks(): Promise<AxiosResponse<DiskResponse>> {
    return apiClient.get<DiskResponse>('/admin/disk-info')
  },

  /**
   * List the subfolders within a given folder.
   */
  getFolders(folder: string): Promise<AxiosResponse<string[]>> {
    return apiClient.get<string[]>('/admin/folders', {
      params: { folder },
    })
  },

  /**
   * Create a new folder.
   */
  makeFolder(data: MakeFolderBody): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/admin/make-folder', data)
  },

  /**
   * Get a sample of media files from a specific folder.
   */
  getMediaSample(folder: string): Promise<AxiosResponse<MediaSampleResponse>> {
    return apiClient.get<MediaSampleResponse>('/admin/media-sample', {
      params: { folder },
    })
  },

  /**
   * Get a list of unsupported files in a specific folder.
   */
  getUnsupportedFiles(folder: string): Promise<AxiosResponse<UnsupportedFilesResponse>> {
    return apiClient.get<UnsupportedFilesResponse>('/admin/unsupported-files', {
      params: { folder },
    })
  },

  // --- Administration Methods ---

  /**
   * Get a list of users for administration.
   */
  getAdminUsers(): Promise<AxiosResponse<AdminUserInfo[]>> {
    return apiClient.get<AdminUserInfo[]>('/admin/users')
  },

  /**
   * Update the media folder for a specific user.
   */
  updateUserMediaFolder(userId: string | number, userFolder: string): Promise<AxiosResponse<void>> {
    return apiClient.put<void>(`/admin/users/${userId}/media-folder`, { userFolder })
  },

  /**
   * Delete a specific user.
   */
  deleteUser(userId: string | number): Promise<AxiosResponse<void>> {
    return apiClient.delete<void>(`/admin/users/${userId}`)
  },

  /**
   * Fetch background jobs list matching filters & sorting parameters.
   */
  getJobs(params: URLSearchParams): Promise<AxiosResponse<PaginatedJobsResponse>> {
    return apiClient.get<PaginatedJobsResponse>('/jobs', { params })
  },

  /**
   * Cancel a specific background job.
   */
  cancelJob(jobId: number): Promise<AxiosResponse<void>> {
    return apiClient.post<void>(`/jobs/${jobId}/cancel`)
  },

  /**
   * Retry a specific background job.
   */
  retryJob(jobId: number): Promise<AxiosResponse<void>> {
    return apiClient.post<void>(`/jobs/${jobId}/retry`)
  },
}

export default adminService
