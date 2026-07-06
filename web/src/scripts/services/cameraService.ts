import apiClient from './api.ts'
import { FullCameraPhotosResponse, ListCameraResponse } from '@/scripts/types/generated/timeline.ts'

const cameraService = {
  async list(): Promise<ListCameraResponse> {
    const response = await apiClient.get('/camera', {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return ListCameraResponse.decode(buffer)
  },

  async get(make: string, model: string): Promise<FullCameraPhotosResponse> {
    const response = await apiClient.get(`/camera/${make}/${model}`, {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return FullCameraPhotosResponse.decode(buffer)
  },
}

export default cameraService
