import type { AxiosResponse } from 'axios'
import apiClient from '@/scripts/services/api.ts'
import type {
  PaginatedExploreTableResponse,
  ExploreTableParams,
  HistogramResponse,
  VisitedLocation,
} from '@/scripts/types/api/explore.ts'
import { LocationDetailsResponse } from '@/scripts/types/generated/timeline.ts'

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
        indexes: null,
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
   * Get visited places
   */
  getVisitedPlaces(): Promise<AxiosResponse<VisitedLocation[]>> {
    return apiClient.get<VisitedLocation[]>('/explore/locations')
  },

  /**
   * Fetch unified location metadata and media items encoded as protobuf
   */
  async getLocation(locationId: string): Promise<LocationDetailsResponse> {
    const response = await apiClient.get(`/explore/locations/${locationId}`, {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return LocationDetailsResponse.decode(buffer)
  },
}

export default exploreService
