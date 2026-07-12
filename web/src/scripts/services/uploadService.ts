import apiClient from './api.ts'

const uploadService = {
  /**
   * Notifies the backend that a resumable upload has been successfully transferred
   * so that it can be processed and queued for database ingestion.
   */
  notifyComplete(uploadId: string, filename: string) {
    return apiClient.post('/upload/complete', { uploadId, filename })
  },
}

export default uploadService
