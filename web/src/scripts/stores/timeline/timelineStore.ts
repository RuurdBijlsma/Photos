import { defineStore } from 'pinia'
import { computed, ref, shallowRef, triggerRef, watch } from 'vue'
import {
  type TimelineItem,
  TimelineMonthRatios,
  type TimelineRatiosResponse,
} from '@/scripts/types/generated/timeline.ts'
import timelineService from '@/scripts/services/timelineService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { requestIdleCallbackAsync } from '@/scripts/utils.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'

export const useTimelineStore = defineStore('timeline', () => {
  const snackbarStore = useSnackbarsStore()
  const selectionStore = useSelectionStore()
  const viewPhotoStore = useViewPhotoStore()

  const thumbnailSnackSent = ref(false)
  const pendingGoToTop = ref(false)
  const allMonthsPreloaded = ref(false)

  const mediaIdInView = ref<string | null>(null)
  const isInitialized = ref(false)
  const monthRatios = shallowRef<TimelineMonthRatios[]>([])
  const monthItems = shallowRef(new Map<string, TimelineItem[]>())
  const mediaItems = shallowRef<TimelineItem[]>([])
  const mediaItemsMap = shallowRef(new Map<string, TimelineItem>())

  watch(monthItems, () => {
    const resultArr: TimelineItem[] = []
    const resultMap: Map<string, TimelineItem> = new Map()
    const monthIds = [...monthItems.value.keys()].sort((a, b) => (a < b ? 1 : -1))
    for (const monthId of monthIds) {
      const group = monthItems.value.get(monthId)!
      const len = group.length
      for (let i = 0; i < len; i++) {
        const item = group[i]!
        resultArr.push(item)
        resultMap.set(item.id, item)
      }
    }
    mediaItemsMap.value = resultMap
    mediaItems.value = resultArr
    triggerRef(mediaItems)
    // not needed immediately:
    requestIdleCallback(() => triggerRef(mediaItemsMap))
  })
  const mediaItemIds = computed(() => mediaItems.value.map((m) => m.id))
  const totalMediaCount = computed(() => monthRatios.value.reduce((a, b) => a + b.count, 0))

  const monthItemsLoading = new Set<string>()
  let ratiosPromise: Promise<TimelineRatiosResponse> | null = null

  async function fetchMonthRatios() {
    try {
      if (!ratiosPromise) ratiosPromise = timelineService.getTimelineRatios()
      const response = await ratiosPromise
      monthRatios.value = response.months
    } catch (e) {
      snackbarStore.error('Failed to fetch timeline layout.', e)
    } finally {
      ratiosPromise = null
    }
  }

  async function fetchMediaByMonth(monthIds: string[], useCache = true) {
    monthIds = monthIds.filter(
      (m) => (!useCache || !monthItems.value.has(m)) && !monthItemsLoading.has(m),
    )
    if (monthIds.length === 0) return
    monthIds.forEach((m) => monthItemsLoading.add(m))

    try {
      const response = await timelineService.getMediaByMonths(monthIds)
      for (const { monthId, items } of response.months) {
        monthItems.value.set(monthId, items)
      }
      triggerRef(monthItems)
    } catch (e) {
      snackbarStore.error('Failed to fetch media.', e)
    } finally {
      monthIds.forEach((m) => monthItemsLoading.delete(m))
    }
  }

  async function setViewPhotoStoreIds() {
    const response = await timelineService.getTimelineIds()
    viewPhotoStore.ids = response.data
  }

  let notifiedAboutThumbnails = false
  function notifySlowThumbnails() {
    notifiedAboutThumbnails = true
    let unloadedCount = 0
    let totalCount = 0
    for (const [, groupItems] of monthItems.value) {
      for (const item of groupItems) {
        totalCount++
        if (!item.hasThumbnails) {
          unloadedCount++
        }
      }
    }
    // If more than 1% is not yet loaded, notify about slow thumbnails
    if (unloadedCount / totalCount > 0.01) {
      snackbarStore.enqueue({
        message: `Your photos are still being prepared. Browsing may be slower and thumbnails may load gradually until processing is complete. [${unloadedCount} remaining]`,
        icon: 'mdi-information',
        timeout: -1,
      })
    } else if (unloadedCount > 0) {
      console.warn(
        `Some thumbnails aren't ingested yet: [${unloadedCount} / ${totalCount}] unloaded`,
      )
    } else if (unloadedCount === 0) {
      console.log('All AVIF thumbnails are available')
    }
  }

  let monthPreloadAbortSignal = { aborted: false }
  function abortMonthPreload() {
    monthPreloadAbortSignal.aborted = true
    monthPreloadAbortSignal = { aborted: false }
    return monthPreloadAbortSignal
  }

  async function preLoadAllMonths(
    monthRatios: TimelineMonthRatios[],
    date: Date,
    abortSignal: { aborted: boolean },
  ) {
    const currentMonthIndex = monthRatios.findIndex(
      ({ monthId }) => monthId === date.toISOString().substring(0, 10),
    )
    let i = 0
    let monthIdsToFetch: string[] = []
    let countToFetch = 0
    const BATCH_SIZE = 500

    if (currentMonthIndex !== -1) {
      monthIdsToFetch.push(monthRatios[currentMonthIndex]!.monthId)
      countToFetch += monthRatios[currentMonthIndex]!.count
    }

    while (true) {
      i++
      const beforeIndex = currentMonthIndex - i
      const afterIndex = currentMonthIndex + i
      const fetchMonthRatios: TimelineMonthRatios[] = []

      if (beforeIndex >= 0) fetchMonthRatios.push(monthRatios[beforeIndex]!)
      if (afterIndex < monthRatios.length) fetchMonthRatios.push(monthRatios[afterIndex]!)

      for (const { monthId, count } of fetchMonthRatios) {
        monthIdsToFetch.push(monthId)
        countToFetch += count
      }

      if (
        countToFetch > BATCH_SIZE ||
        (fetchMonthRatios.length === 0 && monthIdsToFetch.length > 0)
      ) {
        await requestIdleCallbackAsync(() => fetchMediaByMonth(monthIdsToFetch))
        countToFetch = 0
        monthIdsToFetch = []
      }

      if (fetchMonthRatios.length === 0 || abortSignal.aborted) {
        if (abortSignal.aborted) {
          console.warn('ABORTED prefetch')
        } else {
          console.log('Fetched all media by month', monthItems.value.keys())
          allMonthsPreloaded.value = true
          selectionStore.allIds = mediaItemIds.value
          if (!notifiedAboutThumbnails) requestIdleCallback(notifySlowThumbnails)
        }
        break
      }
    }
  }

  async function refresh() {
    abortMonthPreload()
    allMonthsPreloaded.value = false
    // Fetch fresh ratios and view-photo ids in parallel.
    await Promise.all([fetchMonthRatios(), setViewPhotoStoreIds()])
    const monthsToFetch = monthRatios.value.map((r) => r.monthId)
    await fetchMediaByMonth(monthsToFetch, false)
  }

  async function initialize() {
    isInitialized.value = true
    await fetchMonthRatios()

    const MONTHS_PREFETCH_COUNT = 2
    const monthsToFetch = monthRatios.value
      .slice(0, MONTHS_PREFETCH_COUNT)
      .map((m) => m?.monthId)
      .filter(Boolean)

    if (monthsToFetch.length > 0) await fetchMediaByMonth(monthsToFetch)
  }

  function scrollToTop() {
    pendingGoToTop.value = true
  }

  return {
    monthRatios,
    monthItems,
    mediaItems,
    mediaItemsMap,
    mediaItemIds,
    totalMediaCount,
    isInitialized,
    mediaIdInView,
    thumbnailSnackSent,
    pendingGoToTop,
    allMonthsPreloaded,

    initialize,
    scrollToTop,
    setViewPhotoStoreIds,
    refresh,
    abortMonthPreload,
    preLoadAllMonths,
  }
})
