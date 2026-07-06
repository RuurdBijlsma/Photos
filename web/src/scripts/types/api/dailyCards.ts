import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'

export interface DailyCardResponse {
  id: number
  cardDate: string | null // NaiveDate
  cardType: 'cluster' | 'on_this_day' | 'estimatr'
  title: string
  subtitle: string | null
  thumbnailMediaItemId: string | null
  payload: Record<string, unknown>
}

export interface CollectionMediaItem extends SimpleTimelineItem {
  width: number
  height: number
  usePanoramaViewer: boolean
  takenAtLocal?: string
}

export interface CardCollectionPayload {
  mediaItems: CollectionMediaItem[]
}
