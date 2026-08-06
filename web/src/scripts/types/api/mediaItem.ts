export interface UpdateMediaItemRequest {
  userCaption?: string | null
  usePanoramaViewer?: boolean
  takenAtLocal?: string
  timezoneOffsetSeconds?: number | null
  orientation?: number
}

export interface UpdateMediaItemResponse {
  mediaItemId: string
}
