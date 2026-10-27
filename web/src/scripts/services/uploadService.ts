import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'

export interface TakeoutStartResponse {
  started: boolean
  zipCount: number
}

const uploadService = {
  getUploadJwt(): Promise<AxiosResponse<string>> {
    return apiClient.get<string>('/upload/jwt')
  },
  startTakeoutImport(): Promise<AxiosResponse<TakeoutStartResponse>> {
    return apiClient.post<TakeoutStartResponse>('/upload/takeout/start')
  },
}

export default uploadService
