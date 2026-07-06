import type { AxiosResponse } from 'axios'
import apiClient from './api'
import type {
  AcceptInviteRequest,
  AddCollaboratorRequest,
  AddMediaToAlbumRequest,
  Album,
  AlbumCollaborator,
  AlbumSort,
  AlbumSortField,
  AlbumSummary,
  BackupInfo,
  CheckInviteRequest,
  CreateAlbumRequest,
  SharedMediaItem,
  SortDirection,
  UpdateAlbumRequest,
} from '@/scripts/types/api/album'
import { FullAlbumMediaResponse, OrderedMediaResponse } from '@/scripts/types/generated/timeline.ts'

const albumService = {
  getUserAlbums(
    sortField: AlbumSortField = 'updatedAt',
    sortDirection: SortDirection = 'desc',
  ): Promise<AxiosResponse<Album[]>> {
    return apiClient.get<Album[]>('/album', { params: { sortField, sortDirection } })
  },

  createAlbum(payload: CreateAlbumRequest): Promise<AxiosResponse<Album>> {
    return apiClient.post<Album>('/album', payload)
  },

  deleteAlbum(albumId: string): Promise<AxiosResponse<Album>> {
    return apiClient.delete<Album>(`/album/${albumId}`)
  },

  updateAlbum(albumId: string, payload: UpdateAlbumRequest): Promise<AxiosResponse<Album>> {
    return apiClient.put<Album>(`/album/${albumId}`, payload)
  },

  async getSortedMedia(albumId: string, sortMode: AlbumSort): Promise<OrderedMediaResponse> {
    const response = await apiClient.get(`/album/${albumId}/media/sorted`, {
      params: { sortMode },
      responseType: 'arraybuffer',
    })
    const buffer = new Uint8Array(response.data)
    return OrderedMediaResponse.decode(buffer)
  },

  reorderMedia(
    albumId: string,
    mediaItemIds: string[],
    sortMode: AlbumSort,
  ): Promise<AxiosResponse<void>> {
    return apiClient.put<void>(`/album/${albumId}/media/reorder`, { mediaItemIds, sortMode })
  },

  // --- Media Management ---

  addMediaToAlbum(albumId: string, payload: AddMediaToAlbumRequest): Promise<AxiosResponse<void>> {
    return apiClient.post<void>(`/album/${albumId}/media`, payload)
  },

  removeMediaFromAlbum(albumId: string, mediaItemIds: string[]): Promise<AxiosResponse<void>> {
    return apiClient.delete<void>(`/album/${albumId}/media/${mediaItemIds.join(',')}`)
  },

  // --- Collaborator Management ---

  addCollaborator(
    albumId: string,
    payload: AddCollaboratorRequest,
  ): Promise<AxiosResponse<AlbumCollaborator>> {
    return apiClient.post<AlbumCollaborator>(`/album/${albumId}/collaborators`, payload)
  },

  /**
   * Remove a collaborator from an album.
   * Note: collaboratorId is the numeric ID of the link record, not the user's ID.
   */
  removeCollaborator(albumId: string, collaboratorId: number): Promise<AxiosResponse<void>> {
    return apiClient.delete<void>(`/album/${albumId}/collaborators/${collaboratorId}`)
  },

  // --- Invite / Sharing System ---

  /**
   * Generate a cross-server invitation link (token) for an album.
   */
  generateInvite(albumId: string): Promise<AxiosResponse<string>> {
    return apiClient.get<string>(`/album/${albumId}/invite`)
  },

  /**
   * Check an invite token to see what album it points to before accepting.
   */
  checkInvite(payload: CheckInviteRequest): Promise<AxiosResponse<AlbumSummary>> {
    return apiClient.post<AlbumSummary>('/album/invite/check', payload)
  },

  /**
   * Accept an invitation token to import the album.
   */
  acceptInvite(payload: AcceptInviteRequest): Promise<AxiosResponse<Album>> {
    return apiClient.post<Album>('/album/invite/accept', payload)
  },

  async getAlbumMedia(albumId: string): Promise<FullAlbumMediaResponse> {
    const response = await apiClient.get(`/album/${albumId}`, {
      responseType: 'arraybuffer',
      params: {
        sort: 'desc',
      },
    })
    const buffer = new Uint8Array(response.data)
    return FullAlbumMediaResponse.decode(buffer)
  },

  getSharedMediaItem(
    albumId: string,
    mediaItemId: string,
  ): Promise<AxiosResponse<SharedMediaItem>> {
    return apiClient.get<SharedMediaItem>(`/album/${albumId}/item/${mediaItemId}`)
  },

  listBackups(): Promise<AxiosResponse<BackupInfo[]>> {
    return apiClient.get<BackupInfo[]>('/album/backup/list')
  },

  restoreBackup(backupFilename: string): Promise<AxiosResponse<void>> {
    return apiClient.post<void>(`/album/restore/${backupFilename}`)
  },
}

export default albumService
