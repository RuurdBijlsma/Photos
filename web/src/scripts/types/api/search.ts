export interface SearchFilterRanges {
  availableMonths: string[]
  people: string[][]
  countries: string[][]
}

export interface BaseSearchParams {
  limit?: number
  offset?: number
  startDate?: string
  endDate?: string
  mediaType?: string
  sortBy?: string
  negativeQuery?: string
  countryCodes?: string
  personIds?: string
  allFacesRequired?: boolean
}

export interface SearchParams extends BaseSearchParams {
  query: string
}
