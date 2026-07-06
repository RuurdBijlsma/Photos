import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'

export const useViewPhotoStore = defineStore('viewPhoto', () => {
  const viewLink = ref<string>('')
  const ids = shallowRef<string[]>([])
  const playMotionTrigger = ref(0)

  function triggerPlayMotion() {
    playMotionTrigger.value++
  }

  return {
    viewLink,
    ids,
    playMotionTrigger,
    triggerPlayMotion,
  }
})
