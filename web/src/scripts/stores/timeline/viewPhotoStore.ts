import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import type {
  SimpleTimelineItem,
  StorageReviewItem,
  TimelineItem,
} from '@/scripts/types/generated/timeline.ts'
import { type LocationQuery, useRouter } from 'vue-router'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { useRefreshStore } from '@/scripts/stores/refreshStore.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import { isMimeTypeSupported } from '@/scripts/utils.ts'

const rotations: Record<number, [number, number, number, number]> = {
  1: [1, 6, 3, 8],
  2: [2, 5, 4, 7],
  3: [3, 8, 1, 6],
  4: [4, 7, 2, 5],
  5: [5, 4, 7, 2],
  6: [6, 3, 8, 1],
  7: [7, 2, 5, 4],
  8: [8, 1, 6, 3],
}

function rotatedOrientationByDegrees(orientation: number, degrees: 0 | 90 | 180 | 270): number {
  return rotations[orientation]?.[degrees / 90] ?? orientation
}

export const useViewPhotoStore = defineStore('viewPhoto', () => {
  const snackbarStore = useSnackbarsStore()
  const refreshStore = useRefreshStore()
  const mediaItemStore = useMediaItemStore()
  const router = useRouter()

  const viewLink = ref<string>('')
  const ids = shallowRef<string[]>([])
  const idsMetadata = shallowRef(
    new Map<string, SimpleTimelineItem | TimelineItem | StorageReviewItem>(),
  )
  const playMotionTrigger = ref(0)
  const rotatedPhotos = ref(new Map<string, number>())
  const rotationLoading = ref(false)
  const rotateDebounceTimers = new Map<string, ReturnType<typeof setTimeout>>()
  const preloadedBlobs = ref(new Map<string, string>())
  const hideRotatedThumb = ref(new Set<string>())

  function triggerPlayMotion() {
    playMotionTrigger.value++
  }

  async function rotatePhoto(
    mediaItemId: string,
    currentOrientation: number | undefined,
    currentRouteQuery: LocationQuery = {},
  ) {
    if (rotationLoading.value) return
    currentOrientation = currentOrientation ?? 1

    const currentRotation = rotatedPhotos.value.get(mediaItemId) ?? 0
    const newRotation = ((currentRotation + 90) % 360) as 0 | 90 | 180 | 270
    rotatedPhotos.value.set(mediaItemId, newRotation)

    // Debounced rotate update on server
    const newOrientation = rotatedOrientationByDegrees(currentOrientation, newRotation)
    if (rotateDebounceTimers.has(mediaItemId)) clearTimeout(rotateDebounceTimers.get(mediaItemId))
    const timer = setTimeout(() => {
      rotateDebounceTimers.delete(mediaItemId)
      rotatePhotoServerSide(mediaItemId, currentOrientation, newOrientation, currentRouteQuery)
    }, 2500)

    rotateDebounceTimers.set(mediaItemId, timer)
  }

  async function rotatePhotoServerSide(
    mediaItemId: string,
    currentOrientation: number,
    newOrientation: number,
    currentRouteQuery: LocationQuery,
  ) {
    if (currentOrientation === newOrientation) return
    rotationLoading.value = true
    let preloadedUrl: string | null = null
    let newId: string | null = null

    try {
      const { data } = await mediaItemService.update(mediaItemId, { orientation: newOrientation })
      newId = data.mediaItemId

      // Fetch updated media item metadata
      await mediaItemStore.fetchMediaItem(newId, false)
      const newItem = mediaItemStore.mediaItems.get(newId)!

      // Pre-download and decode the rotated full-res image blob
      if (isMimeTypeSupported(newItem.media_features.mime_type))
        try {
          const res = await mediaItemService.downloadMediaFileById(newId)
          preloadedUrl = URL.createObjectURL(res.data)
          const img = new Image()
          img.src = preloadedUrl
          await img.decode().catch(() => {})
          preloadedBlobs.value.set(newId, preloadedUrl)
        } catch (err) {
          console.warn('Failed to preload rotated full res blob:', err)
        }

      hideRotatedThumb.value.add(newId)
      hideRotatedThumb.value.add(mediaItemId)
      // Update route if viewing the rotated item
      if (router.currentRoute.value.params.mediaId === mediaItemId) {
        await router.replace({
          path: `${viewLink.value}${newId}`,
          query: currentRouteQuery,
        })
      }
      refreshStore.counter++
    } catch (e) {
      if (preloadedUrl) URL.revokeObjectURL(preloadedUrl)
      snackbarStore.error('Could not rotate photo', e)
    } finally {
      rotationLoading.value = false
      setTimeout(() => {
        rotatedPhotos.value.delete(mediaItemId)
        hideRotatedThumb.value.delete(mediaItemId)
        if (newId) hideRotatedThumb.value.delete(newId)
      }, 250)
    }
  }

  return {
    viewLink,
    ids,
    idsMetadata,
    playMotionTrigger,
    rotatedPhotos,
    rotationLoading,
    preloadedBlobs,
    hideRotatedThumb,
    triggerPlayMotion,
    rotatePhoto,
  }
})
