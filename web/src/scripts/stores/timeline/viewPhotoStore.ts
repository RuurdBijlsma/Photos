import { defineStore } from 'pinia'
import { ref, shallowRef, triggerRef } from 'vue'
import type {
  SimpleTimelineItem,
  StorageReviewItem,
  TimelineItem,
} from '@/scripts/types/generated/timeline.ts'
import type { LocationQuery } from 'vue-router'

export const useViewPhotoStore = defineStore('viewPhoto', () => {
  const viewLink = ref<string>('')
  const ids = shallowRef<string[]>([])
  const idsMetadata = shallowRef(
    new Map<string, SimpleTimelineItem | TimelineItem | StorageReviewItem>(),
  )
  const playMotionTrigger = ref(0)
  const rotatedPhotos = ref(new Map<string, number>())

  function triggerPlayMotion() {
    playMotionTrigger.value++
  }

  function rotatePhoto(id: string, currentOrientation: number, currentRoute: LocationQuery) {
    console.log('rotatePhoto', id, currentOrientation, currentRoute)
    const currentRotation = rotatedPhotos.value.get(id) ?? 0
    rotatedPhotos.value.set(id, (currentRotation + 90) % 360)
    console.log('rotated:', rotatedPhotos.value.get(id))
  }

  return {
    viewLink,
    ids,
    idsMetadata,
    playMotionTrigger,
    rotatedPhotos,
    triggerPlayMotion,
    rotatePhoto,
  }
})
