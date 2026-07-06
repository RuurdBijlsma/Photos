import apiClient from './api.ts'
import { SearchResponse, SearchSuggestionsResponse } from '@/scripts/types/generated/timeline.ts'
import type { AxiosResponse } from 'axios'
import type { SearchFilterRanges, SearchParams } from '@/scripts/types/api/search.ts'

const searchService = {
  async search(params: SearchParams, signal?: AbortSignal): Promise<SearchResponse> {
    console.log('Search with params', params)
    const response = await apiClient.get('/search', {
      params,
      responseType: 'arraybuffer',
      signal,
    })
    const buffer = new Uint8Array(response.data)
    return SearchResponse.decode(buffer)
  },

  async suggestions(query: string, limit?: number): Promise<SearchSuggestionsResponse> {
    const response = await apiClient.get('/search/suggestions', {
      params: { query, limit },
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return SearchSuggestionsResponse.decode(buffer)
  },

  randomSuggestion(): Promise<AxiosResponse<string>> {
    return apiClient.get<string>('/search/suggestions/random')
  },

  filterRanges(): Promise<AxiosResponse<SearchFilterRanges>> {
    return apiClient.get<SearchFilterRanges>('/search/params')
  },

  async searchByImage(
    imageFile: File,
    params: SearchParams,
    signal?: AbortSignal,
  ): Promise<SearchResponse> {
    const formData = new FormData()
    formData.append('image', imageFile)

    const response = await apiClient.post('/search/image', formData, {
      params,
      responseType: 'arraybuffer',
      headers: {
        'Content-Type': undefined,
      },
      signal,
    })
    const buffer = new Uint8Array(response.data)
    return SearchResponse.decode(buffer)
  },

  async searchByImageSession(
    sessionId: string,
    params: SearchParams,
    signal?: AbortSignal,
  ): Promise<SearchResponse> {
    const response = await apiClient.get(`/search/image/uuid/${sessionId}`, {
      params,
      responseType: 'arraybuffer',
      signal,
    })
    const buffer = new Uint8Array(response.data)
    return SearchResponse.decode(buffer)
  },
}

export default searchService
