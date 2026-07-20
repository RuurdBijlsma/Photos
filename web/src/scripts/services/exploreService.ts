import type { AxiosResponse } from 'axios'
import apiClient from '@/scripts/services/api.ts'
import type {
  PaginatedExploreTableResponse,
  ExploreTableParams,
  HistogramResponse,
  VisitedPlacesResponse,
  VisitedLocation,
} from '@/scripts/types/api/explore.ts'
import { OrderedMediaResponse } from '@/scripts/types/generated/timeline.ts'

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

  /**
   * Get categorized visited places
   */
  getVisitedPlaces(): Promise<AxiosResponse<VisitedPlacesResponse>> {
    return apiClient.get<VisitedPlacesResponse>('/explore/locations')
  },

  /**
   * Fetch location metadata details
   */
  getLocationDetails(locationId: number): Promise<AxiosResponse<VisitedLocation>> {
    return apiClient.get<VisitedLocation>(`/explore/locations/${locationId}/details`)
  },

  /**
   * Fetch chronological simple timeline items encoded as protobuf
   */
  async getLocationMedia(locationId: number): Promise<OrderedMediaResponse> {
    const response = await apiClient.get(`/explore/locations/${locationId}/media`, {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return OrderedMediaResponse.decode(buffer)
  },
}

export default exploreService
