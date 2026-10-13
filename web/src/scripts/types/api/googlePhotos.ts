export interface GooglePhotosConfigStatus {
  configured: boolean
  clientId?: string
}

export interface GooglePhotosStatus {
  configured: boolean
  clientId?: string
}

export interface SaveGoogleConfigRequest {
  clientId: string
  clientSecret: string
}

export interface ExchangeCodeRequest {
  code: string
  redirectUri: string
}

export interface StartPickerResponse {
  importSessionId: string
  pickerUri: string
}

export interface MatchedItem {
  mediaItemId: string
  filename: string
}

export interface UnmatchedItem {
  filename: string
}

export interface PollResult {
  ready: boolean
  matched: MatchedItem[]
  unmatched: UnmatchedItem[]
}

export interface CommitImportRequest {
  importSessionId: string
  albumName: string
  albumDescription?: string
  mediaItemIds: string[]
}
