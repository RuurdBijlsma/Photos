import apiClient from './api.ts'
import type { AxiosResponse } from 'axios'
import type { DailyCardResponse } from '@/scripts/types/api/dailyCards.ts'

const dailyCardService = {
  getCards(iso_date: string | undefined): Promise<AxiosResponse<DailyCardResponse[]>> {
    return apiClient.get<DailyCardResponse[]>('/daily-cards', {
      params: { date: iso_date },
    })
  },

  validateMedia(mediaItemIds: string[]): Promise<AxiosResponse<string[]>> {
    return apiClient.post<string[]>('/daily-cards/validate-media', { mediaItemIds })
  },
}

export default dailyCardService
