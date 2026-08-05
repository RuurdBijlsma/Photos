import type { AxiosResponse } from 'axios'
import apiClient, { SERVER_BASE_URL } from './api.ts'
import type { RandomPhotoResponse } from '@/scripts/types/api/photos.ts'
import type { MediaItemWithAlbums } from '@/scripts/types/api/fullPhoto.ts'
import type { Theme } from '@/scripts/types/themeColor.ts'
import type {
  UpdateMediaItemRequest,
  UpdateMediaItemResponse,
} from '@/scripts/types/api/mediaItem.ts'
import { MapPhotosResponse } from '@/scripts/types/generated/timeline.ts'
import type { ThemeVariant } from '@/scripts/constants.ts'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

const mediaItemService = {
  update(id: string, payload: UpdateMediaItemRequest) {
    return apiClient.put<UpdateMediaItemResponse>(`/photos/${id}/item`, payload)
  },

  getPhotoThumbnail(
    id: string | null | undefined,
    size: number,
    onDemand: boolean | undefined,
  ): string {
    if (!id) return ''
    return onDemand
      ? `${SERVER_BASE_URL}/api/photos/${id}/thumbnail?size=${size}`
      : `${SERVER_BASE_URL}/thumbnails/${id}/${size}p.avif`
  },

  getVideo(id: string | null | undefined, size: number, onDemand: boolean | undefined): string {
    if (!id) return ''
    return onDemand
      ? `${SERVER_BASE_URL}/api/photos/${id}/video`
      : `${SERVER_BASE_URL}/thumbnails/${id}/${size}p.webm`
  },

  getMotionVideo(id: string | null | undefined): string {
    if (!id) return ''
    return `${SERVER_BASE_URL}/thumbnails/${id}/motion.mp4`
  },

  getRandomPhoto(
    variant: ThemeVariant = 'Expressive',
    contrast: number = 0.2,
  ): Promise<AxiosResponse<RandomPhotoResponse | null>> {
    return apiClient.get<RandomPhotoResponse | null>('/theme/random-photo', {
      params: { variant, contrast },
    })
  },

  getTheme(
    color: string,
    variant: ThemeVariant = 'Expressive',
    contrast: number = 0.2,
  ): Promise<AxiosResponse<Theme>> {
    return apiClient.get<Theme>('/theme/by-color', {
      params: { color, variant, contrast },
    })
  },

  getMediaItem(id: string): Promise<AxiosResponse<MediaItemWithAlbums>> {
    return apiClient.get<MediaItemWithAlbums>(`/photos/${id}/item`)
  },

  downloadMediaFile(relative_path: string): Promise<AxiosResponse<Blob>> {
    return apiClient.get<Blob>('/photos/download', {
      params: { path: relative_path },
      responseType: 'blob',
    })
  },

  downloadMediaFileById(id: string, signal?: AbortSignal): Promise<AxiosResponse<Blob>> {
    return apiClient.get<Blob>(`/photos/${id}/download`, {
      responseType: 'blob',
      signal,
    })
  },

  downloadMediaZip(ids: string[]): Promise<AxiosResponse<Blob>> {
    return apiClient.get<Blob>('/photos/download/zip', {
      params: { ids: ids.join(',') },
      responseType: 'blob',
    })
  },

  async listMapPhotos(startDate?: string, endDate?: string): Promise<MapPhotosResponse> {
    const response = await apiClient.get('/photos/geo', {
      responseType: 'arraybuffer',
      params: { startDate, endDate },
    })
    const buffer = new Uint8Array(response.data)
    return MapPhotosResponse.decode(buffer)
  },

  async getPanoConfig(id: string): Promise<AxiosResponse<PannellumConfig>> {
    return apiClient.get<PannellumConfig>(`/photos/${id}/pano-config`)
  },
}

export default mediaItemService
