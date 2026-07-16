import type { AxiosResponse } from 'axios'
import apiClient from '@/scripts/services/api.ts'
import type {
  PaginatedExploreTableResponse,
  ExploreTableParams,
} from '@/scripts/types/api/explore.ts'

const exploreService = {
  /**
   * Get paginated media items with stats for the explore table.
   */
  getExploreTable(
    params?: ExploreTableParams,
  ): Promise<AxiosResponse<PaginatedExploreTableResponse>> {
    const searchParams = new URLSearchParams()
    if (params) {
      if (params.page !== undefined) searchParams.append('page', params.page.toString())
      if (params.limit !== undefined) searchParams.append('limit', params.limit.toString())
      if (params.offset !== undefined) searchParams.append('offset', params.offset.toString())
      if (params.sort) {
        params.sort.forEach((s) => searchParams.append('sort', s))
      }
    }
    return apiClient.get<PaginatedExploreTableResponse>('/explore/table', { params: searchParams })
  },
}

export default exploreService
