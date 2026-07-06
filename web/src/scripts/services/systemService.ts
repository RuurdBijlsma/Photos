import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'
import type { SystemStats } from '@/scripts/types/api/system.ts'

const systemService = {
  getStats(): Promise<AxiosResponse<SystemStats>> {
    return apiClient.get<SystemStats>(`/system/stats`)
  },
}

export default systemService
