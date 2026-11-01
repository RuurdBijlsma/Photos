export interface UpdateMediaItemRequest {
  userCaption?: string | null
  usePanoramaViewer?: boolean
  takenAtLocal?: string
  timezoneOffsetSeconds?: number | null
  orientation?: number
  gps?: { latitude: number; longitude: number }
}

export interface UpdateMediaItemResponse {
  mediaItemId: string
}
