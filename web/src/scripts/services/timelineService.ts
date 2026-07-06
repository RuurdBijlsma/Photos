import type { AxiosResponse } from 'axios'
import apiClient from './api.ts'

import {
  TimelineItemsResponse,
  TimelineRatiosResponse,
} from '@/scripts/types/generated/timeline.ts'

const timelineService = {
  getTimelineIds(): Promise<AxiosResponse<string[]>> {
    return apiClient.get<string[]>('/timeline/ids', { params: { sort: 'desc' } })
  },

  async getTimelineRatios(): Promise<TimelineRatiosResponse> {
    const response = await apiClient.get('/timeline/ratios', {
      params: { sort: 'desc' },
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return TimelineRatiosResponse.decode(buffer)
  },

  async getMediaByMonths(months: string[]): Promise<TimelineItemsResponse> {
    const response = await apiClient.get('/timeline/by-month', {
      responseType: 'arraybuffer',
      params: {
        sort: 'desc',
        months: months.join(','),
      },
    })
    const buffer = new Uint8Array(response.data)
    return TimelineItemsResponse.decode(buffer)
  },
}

export default timelineService
