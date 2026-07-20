import type { AxiosResponse } from 'axios'
import apiClient from '@/scripts/services/api.ts'
import type {
  PaginatedExploreTableResponse,
  ExploreTableParams,
  HistogramResponse,
} from '@/scripts/types/api/explore.ts'

const exploreService = {
  /**
   * Get paginated media items with stats for the explore table.
   */
  getExploreTable(
    params?: ExploreTableParams,
  ): Promise<AxiosResponse<PaginatedExploreTableResponse>> {
    return apiClient.get<PaginatedExploreTableResponse>('/explore/table', {
      params,
      paramsSerializer: {
        indexes: null, // Forces Axios to repeat array params directly (e.g., sort=a&sort=b)
      },
    })
  },

  /**
   * Get media item histogram insights.
   */
  getHistograms(): Promise<AxiosResponse<HistogramResponse>> {
    return apiClient.get<HistogramResponse>('/explore/histograms')
  },
}

export default exploreService
