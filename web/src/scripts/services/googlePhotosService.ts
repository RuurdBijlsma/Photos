import type { AxiosResponse } from 'axios'
import apiClient from './api'
import type {
  CommitImportRequest,
  ExchangeCodeRequest,
  GooglePhotosConfigStatus,
  PollResult,
  SaveGoogleConfigRequest,
  StartPickerResponse,
} from '@/scripts/types/api/googlePhotos'
import type { Album } from '@/scripts/types/api/album'

const googlePhotosService = {
  /** Get whether the admin has configured Google OAuth credentials. */
  getConfigStatus(): Promise<AxiosResponse<GooglePhotosConfigStatus>> {
    return apiClient.get<GooglePhotosConfigStatus>('/admin/google-photos/config')
  },

  /** Save (overwrite) the Google OAuth client credentials (admin only). */
  saveConfig(payload: SaveGoogleConfigRequest): Promise<AxiosResponse<void>> {
    return apiClient.post<void>('/admin/google-photos/config', payload)
  },

  /**
   * Exchange an OAuth authorization code, create a Google Picker session, and
   * return the picker URI to open in a popup.
   */
  startPicker(payload: ExchangeCodeRequest): Promise<AxiosResponse<StartPickerResponse>> {
    return apiClient.post<StartPickerResponse>('/google-photos/start', payload)
  },

  /** Poll an import session — returns ready status and matched/unmatched items. */
  pollSession(importSessionId: string): Promise<AxiosResponse<PollResult>> {
    return apiClient.get<PollResult>(`/google-photos/poll/${importSessionId}`)
  },

  /** Commit: create a local album from the confirmed matched items. */
  commitImport(payload: CommitImportRequest): Promise<AxiosResponse<Album>> {
    return apiClient.post<Album>('/google-photos/commit', payload)
  },
}

export default googlePhotosService
