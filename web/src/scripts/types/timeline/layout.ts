import type { AlbumInfo, SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'

export type TimelineContext = { album?: AlbumInfo; isBin?: boolean }

export interface SimpleLayoutRow {
  items: SimpleTimelineItem[]
  height: number
  key: string
  offsetTop: number
  thumbnailSize: number
  firstRow: boolean
  lastRow: boolean
}

export interface LayoutRow {
  items: LayoutRowItem[]
  height: number
  date: Date
  monthId: string
  firstOfTheMonth: boolean
  lastOfTheMonth: boolean
  key: string
  offsetTop: number
  thumbnailSize: number
}

export interface LayoutRowItem {
  ratio: number
  index: number
}
