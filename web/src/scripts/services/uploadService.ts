import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'

const uploadService = {
  getUploadJwt(): Promise<AxiosResponse<string>> {
    return apiClient.get<string>('/upload/jwt')
  },
}

export default uploadService
