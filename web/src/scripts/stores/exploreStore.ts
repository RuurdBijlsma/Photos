import { ref, type Ref, shallowRef, triggerRef } from 'vue'
import { defineStore } from 'pinia'
import type {
  ExploreMediaItem,
  HistogramResponse,
  VisitedLocation,
} from '@/scripts/types/api/explore.ts'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import exploreService from '@/scripts/services/exploreService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'

export const useExploreStore = defineStore('explore', () => {
  const snackbarStore = useSnackbarsStore()

  // --- STATE ---
  const items: Ref<ExploreMediaItem[]> = shallowRef([])
  const totalCount = ref(0)
  const isTableLoading = ref(false)

  // Histograms STATE
  const histograms: Ref<HistogramResponse | null> = shallowRef(null)
  const isHistogramsLoading = ref(false)

  // Visited Places & Details STATE
  const visitedPlaces: Ref<VisitedLocation[] | null> = shallowRef(null)
  const isVisitedPlacesLoading = ref(false)
  const locationMedia = shallowRef(new Map<number, SimpleTimelineItem[]>())
  const locationDetails = shallowRef(new Map<number, VisitedLocation>())
  const isLocationLoading = ref(false)

  // Pagination & Datatable Parameters
  const page = ref(1)
  const itemsPerPage = ref(15)
  const sortBy: Ref<{ key: string; order: 'asc' | 'desc' }[]> = shallowRef([])

  // --- ACTIONS ---
  async function fetchExploreTable() {
    isTableLoading.value = true
    try {
      const sortParams: string[] = []
      if (sortBy.value && sortBy.value.length > 0) {
        sortBy.value.forEach((s) => {
          sortParams.push(`${s.key}:${s.order}`)
        })
      }

      const response = await exploreService.getExploreTable({
        page: page.value,
        limit: itemsPerPage.value,
        sort: sortParams,
      })

      items.value = response.data.data
      totalCount.value = response.data.total
    } catch (error) {
      snackbarStore.error('Failed to load explore stats', error)
    } finally {
      isTableLoading.value = false
    }
  }

  async function fetchHistograms() {
    isHistogramsLoading.value = true
    try {
      const response = await exploreService.getHistograms()
      histograms.value = response.data
    } catch (error) {
      snackbarStore.error('Failed to load explore histograms', error)
    } finally {
      isHistogramsLoading.value = false
    }
  }

  async function fetchVisitedPlaces() {
    isVisitedPlacesLoading.value = true
    try {
      const response = await exploreService.getVisitedPlaces()
      visitedPlaces.value = response.data
    } catch (error) {
      snackbarStore.error('Failed to load visited places', error)
    } finally {
      isVisitedPlacesLoading.value = false
    }
  }

  async function fetchLocationData(locationId: number) {
    isLocationLoading.value = true
    try {
      const [media, details] = await Promise.all([
        exploreService.getLocationMedia(locationId),
        exploreService.getLocationDetails(locationId),
      ])
      locationMedia.value.set(locationId, media.items)
      locationDetails.value.set(locationId, details.data)
      triggerRef(locationMedia)
      triggerRef(locationDetails)
    } catch (error) {
      snackbarStore.error('Failed to load location data', error)
    } finally {
      isLocationLoading.value = false
    }
  }

  function resetPagination() {
    page.value = 1
    items.value = []
    totalCount.value = 0
  }

  return {
    items,
    totalCount,
    isTableLoading,
    histograms,
    isHistogramsLoading,
    visitedPlaces,
    isVisitedPlacesLoading,
    locationMedia,
    locationDetails,
    isLocationLoading,
    page,
    itemsPerPage,
    sortBy,
    fetchExploreTable,
    fetchHistograms,
    fetchVisitedPlaces,
    fetchLocationData,
    resetPagination,
  }
})
