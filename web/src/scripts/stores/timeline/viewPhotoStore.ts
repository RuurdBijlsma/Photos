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
import { useDebounceFn } from '@vueuse/core'
import { useRefreshStore } from '@/scripts/stores/refreshStore.ts'

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
  const router = useRouter()

  const viewLink = ref<string>('')
  const ids = shallowRef<string[]>([])
  const idsMetadata = shallowRef(
    new Map<string, SimpleTimelineItem | TimelineItem | StorageReviewItem>(),
  )
  const playMotionTrigger = ref(0)
  const rotatedPhotos = ref(new Map<string, number>())
  const rotationLoading = ref(false)

  function triggerPlayMotion() {
    playMotionTrigger.value++
  }

  async function rotatePhoto(
    mediaItemId: string,
    currentOrientation: number,
    currentRouteQuery: LocationQuery,
  ) {
    if (rotationLoading.value) return
    const currentRotation = rotatedPhotos.value.get(mediaItemId) ?? 0
    const newRotation = ((currentRotation + 90) % 360) as 90 | 180 | 270
    rotatedPhotos.value.set(mediaItemId, newRotation)
    const newOrientation = rotatedOrientationByDegrees(currentOrientation, newRotation)
    debouncedRotatePhotoServerSide(
      mediaItemId,
      currentOrientation,
      newOrientation,
      currentRouteQuery,
    )
  }

  const debouncedRotatePhotoServerSide = useDebounceFn(rotatePhotoServerSide, 2500)

  async function rotatePhotoServerSide(
    mediaItemId: string,
    currentOrientation: number,
    newOrientation: number,
    currentRouteQuery: LocationQuery,
  ) {
    if (currentOrientation === newOrientation) return
    rotationLoading.value = true
    try {
      const { data } = await mediaItemService.update(mediaItemId, { orientation: newOrientation })
      const newId = data.mediaItemId
      if (router.currentRoute.value.params.mediaId === mediaItemId) {
        await router.replace({
          path: `${viewLink.value}${newId}`,
          query: currentRouteQuery,
        })
      }
      refreshStore.counter++
    } catch (e) {
      snackbarStore.error('Could not rotate photo', e)
    } finally {
      rotationLoading.value = false
      rotatedPhotos.value.delete(mediaItemId)
      console.log('NICE')
    }
  }

  return {
    viewLink,
    ids,
    idsMetadata,
    playMotionTrigger,
    rotatedPhotos,
    rotationLoading,
    triggerPlayMotion,
    rotatePhoto,
  }
})
