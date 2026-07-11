import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import type {
  SimpleTimelineItem,
  StorageReviewItem,
  TimelineItem,
} from '@/scripts/types/generated/timeline.ts'

export const useViewPhotoStore = defineStore('viewPhoto', () => {
  const viewLink = ref<string>('')
  const ids = shallowRef<string[]>([])
  const idsMetadata = shallowRef(new Map<string,SimpleTimelineItem | TimelineItem | StorageReviewItem>())
  const playMotionTrigger = ref(0)

  function triggerPlayMotion() {
    playMotionTrigger.value++
  }

  return {
    viewLink,
    ids,
    idsMetadata,
    playMotionTrigger,
    triggerPlayMotion,
  }
})
