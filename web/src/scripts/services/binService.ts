import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import { OrderedMediaResponse } from '@/scripts/types/generated/timeline.ts'

const binService = {
  async getBinTimeline(): Promise<OrderedMediaResponse> {
    const response = await apiClient.get('/trash', {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return OrderedMediaResponse.decode(buffer)
  },

  softDelete(ids: string[]): Promise<AxiosResponse<void>> {
    return apiClient.delete<void>('/trash/soft-delete', { data: { ids } })
  },

  restore(ids: string[]): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/trash/restore', { ids })
  },

  deletePermanently(ids: string[]): Promise<AxiosResponse<void>> {
    return apiClient.delete<void>('/trash/hard-delete', { data: { ids } })
  },
}

export default binService
