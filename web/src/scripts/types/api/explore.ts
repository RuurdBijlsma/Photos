export interface ExploreMediaItem {
  id: string
  filename: string
  isVideo: boolean
  hasThumbnails: boolean
  durationMs: number | null
  takenAtLocal: string

  // GPS fields
  latitude: number | null
  longitude: number | null
  altitude: number | null

  // Weather fields
  temperature: number | null
  windSpeed: number | null
  relativeHumidity: number | null
  precipitation: number | null
  snow: number | null

  // Camera settings
  iso: number | null
  exposureTime: number | null
  aperture: number | null
  focalLength: number | null

  // Media features
  sizeBytes: number | null
}

export interface PaginatedExploreTableResponse {
  data: ExploreMediaItem[]
  total: number
  limit: number
  offset: number
}

export interface ExploreTableParams {
  page?: number
  limit?: number
  offset?: number
  sort?: string[]
}

export interface DayOfWeekBucket {
  day: number
  label: string
  count: number
}

export interface WeekOfYearBucket {
  week: number
  count: number
}

export interface HourOfDayBucket {
  hour: number
  count: number
}

export interface HistogramResponse {
  dayOfWeek: DayOfWeekBucket[]
  weekOfYear: WeekOfYearBucket[]
  hourOfDay: HourOfDayBucket[]
}

export interface VisitedLocation {
  id: string
  name: string
  admin1: string
  admin2: string
  countryCode: string
  countryName: string
  photoCount: number
  thumbnailId: string | null
}
