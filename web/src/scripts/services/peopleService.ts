import apiClient from './api.ts'
import { FullPersonMediaResponse, ListPeopleResponse } from '@/scripts/types/generated/timeline.ts'
import type { AxiosResponse } from 'axios'
import type { MergePersonRequest, UpdatePersonRequest } from '@/scripts/types/api/people.ts'

const peopleService = {
  getFaceThumbnail(clusterId: string | null | undefined): string {
    if (clusterId === null || clusterId === undefined) return ''
    const baseUrl = apiClient.defaults.baseURL
    const path = `/hosted/face-clusters/${clusterId}.webp`
    return new URL(path, baseUrl).href
  },

  getPersonThumbnail(personId: string | null | undefined): string {
    if (personId === null || personId === undefined) return ''
    const baseUrl = apiClient.defaults.baseURL
    const path = `/people/${personId}/thumbnail`
    return new URL(path, baseUrl).href
  },

  async list(): Promise<ListPeopleResponse> {
    const response = await apiClient.get('/people', {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return ListPeopleResponse.decode(buffer)
  },

  async get(personId: string): Promise<FullPersonMediaResponse> {
    const response = await apiClient.get(`/people/${personId}`, {
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return FullPersonMediaResponse.decode(buffer)
  },

  update(personId: string, payload: UpdatePersonRequest): Promise<AxiosResponse<void>> {
    return apiClient.patch(`/people/${personId}`, payload)
  },

  merge(personId: string, payload: MergePersonRequest): Promise<AxiosResponse<void>> {
    return apiClient.post(`/people/${personId}/merge`, payload)
  },

  unmerge(personId: string): Promise<AxiosResponse<void>> {
    return apiClient.post(`/people/${personId}/unmerge`)
  },

  getMediaItemId(personId: string): Promise<AxiosResponse<string>> {
    return apiClient.get<string>(`/people/${personId}/media-item-id`)
  },
}

export default peopleService
