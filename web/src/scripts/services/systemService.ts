import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import type { PublicSystemStats, SystemStats } from '@/scripts/types/api/system.ts'

const systemService = {
  getStats(): Promise<AxiosResponse<SystemStats>> {
    return apiClient.get<SystemStats>(`/system/stats`)
  },

  getPublicStats(): Promise<AxiosResponse<PublicSystemStats>> {
    return apiClient.get<PublicSystemStats>(`/system/public/stats`)
  },
}

export default systemService
