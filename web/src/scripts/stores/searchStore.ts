import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export const useSearchStore = defineStore('search', () => {
  const searchImage = ref<File | null>(null)
  const searchImageSessionId = ref<string | null>(null)
  const imagePreview = ref<string | null>(null)

  watch(
    searchImage,
    (newFile) => {
      searchImageSessionId.value = null
      if (imagePreview.value) {
        URL.revokeObjectURL(imagePreview.value)
        imagePreview.value = null
      }
      if (newFile) {
        imagePreview.value = URL.createObjectURL(newFile)
      }
    },
    { immediate: true },
  )

  return {
    searchImage,
    searchImageSessionId,
    imagePreview,
  }
})
