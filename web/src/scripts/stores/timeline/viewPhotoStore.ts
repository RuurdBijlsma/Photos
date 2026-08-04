import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import type {
  SimpleTimelineItem,
  StorageReviewItem,
  TimelineItem,
} from '@/scripts/types/generated/timeline.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { useRefreshStore } from '@/scripts/stores/refreshStore.ts'
import router from '@/scripts/plugins/router.ts'

export const useViewPhotoStore = defineStore('viewPhoto', () => {
  const viewLink = ref<string>('')
  const ids = shallowRef<string[]>([])
  const idsMetadata = shallowRef(
    new Map<string, SimpleTimelineItem | TimelineItem | StorageReviewItem>(),
  )
  const playMotionTrigger = ref(0)
  const transientRotations = ref<Map<string, number>>(new Map())

  const debounceTimers = new Map<string, number>()
  const pendingClicks = new Map<string, number>()

  function triggerPlayMotion() {
    playMotionTrigger.value++
  }

  function getTransientRotation(mediaItemId: string): number {
    return transientRotations.value.get(mediaItemId) ?? 0
  }

  function rotateOrientationClockwise(orientation: number): number {
    switch (orientation) {
      case 1:
        return 6
      case 6:
        return 3
      case 3:
        return 8
      case 8:
        return 1
      case 2:
        return 5
      case 5:
        return 4
      case 4:
        return 7
      case 7:
        return 2
      default:
        return 6
    }
  }

  function rotatePhoto(
    mediaItemId: string,
    currentExifOrientation: number = 1,
    currentRouteQuery: Record<string, any> = {},
  ) {
    const currentTransient = getTransientRotation(mediaItemId)
    const newTransient = (currentTransient + 90) % 360
    const updatedMap = new Map(transientRotations.value)
    updatedMap.set(mediaItemId, newTransient)
    transientRotations.value = updatedMap

    const prevClicks = pendingClicks.get(mediaItemId) ?? 0
    const newClicks = prevClicks + 1
    pendingClicks.set(mediaItemId, newClicks)

    if (debounceTimers.has(mediaItemId)) {
      clearTimeout(debounceTimers.get(mediaItemId))
    }

    const timer = window.setTimeout(async () => {
      debounceTimers.delete(mediaItemId)
      const totalClicks = pendingClicks.get(mediaItemId) ?? 1
      pendingClicks.delete(mediaItemId)

      let targetOrientation = currentExifOrientation
      for (let i = 0; i < totalClicks; i++) {
        targetOrientation = rotateOrientationClockwise(targetOrientation)
      }

      try {
        const res = await mediaItemService.update(mediaItemId, { orientation: targetOrientation })
        const newId = res.data.mediaItemId
        if (newId && newId !== mediaItemId) {
          const mapToClean = new Map(transientRotations.value)
          mapToClean.delete(mediaItemId)
          mapToClean.delete(newId)
          transientRotations.value = mapToClean

          const idx = ids.value.indexOf(mediaItemId)
          if (idx !== -1) {
            const newIds = [...ids.value]
            newIds[idx] = newId
            ids.value = newIds
          }

          if (idsMetadata.value.has(mediaItemId)) {
            const meta = idsMetadata.value.get(mediaItemId)!
            const newMap = new Map(idsMetadata.value)
            newMap.delete(mediaItemId)
            newMap.set(newId, meta)
            idsMetadata.value = newMap
          }

          if (router.currentRoute.value.params.mediaId === mediaItemId) {
            await router.replace({
              path: `${viewLink.value}${newId}`,
              query: currentRouteQuery,
            })
          }

          const refreshStore = useRefreshStore()
          refreshStore.counter++
        }
      } catch (err) {
        console.error('Failed to save image rotation:', err)
      }
    }, 400)

    debounceTimers.set(mediaItemId, timer)
  }

  return {
    viewLink,
    ids,
    idsMetadata,
    playMotionTrigger,
    transientRotations,
    triggerPlayMotion,
    getTransientRotation,
    rotatePhoto,
  }
})
