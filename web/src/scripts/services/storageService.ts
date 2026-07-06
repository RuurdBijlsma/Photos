import apiClient from './api.ts'
import {
  StorageReviewResponse,
  StorageSummaryResponse,
} from '@/scripts/types/generated/timeline.ts'
import binService from '@/scripts/services/binService.ts'

const storageService = {
  async getSummary(): Promise<StorageSummaryResponse> {
    const response = await apiClient.get('/storage/summary', {
      responseType: 'arraybuffer',
    })
    return StorageSummaryResponse.decode(new Uint8Array(response.data))
  },

  async getReviewItems(): Promise<StorageReviewResponse> {
    const response = await apiClient.get('/storage/review', {
      responseType: 'arraybuffer',
    })
    return StorageReviewResponse.decode(new Uint8Array(response.data))
  },

  async getBlurryItems(): Promise<StorageReviewResponse> {
    const response = await apiClient.get('/storage/blurry', {
      responseType: 'arraybuffer',
    })
    return StorageReviewResponse.decode(new Uint8Array(response.data))
  },

  async deletePermanently(ids: string[]): Promise<void> {
    await binService.softDelete(ids)
    // await binService.deletePermanently(ids)
  },
}

export default storageService
