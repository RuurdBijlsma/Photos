import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useLayoutStore = defineStore('layout', () => {
  const scrollTarget = ref<string | undefined>(undefined)
  const isAppBarVisible = ref(true)

  return { scrollTarget, isAppBarVisible }
})
