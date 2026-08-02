import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { useTitle } from '@vueuse/core'
import { APP_NAME } from '@/scripts/constants.ts'

export const useTitleStore = defineStore('title', () => {
  const pageTitle = ref<string | null>(null)
  const detailTitle = ref<string | null>(null)

  const fullTitle = computed(() => {
    const parts = [detailTitle.value, pageTitle.value, APP_NAME].filter((part): part is string =>
      Boolean(part && part.trim()),
    )
    return parts.join(' - ')
  })

  // VueUse useTitle reactively updates document.title
  useTitle(fullTitle)

  function setPageTitle(title: string | null) {
    pageTitle.value = title
  }

  function setDetailTitle(title: string | null) {
    detailTitle.value = title
  }

  function resetTitles() {
    pageTitle.value = null
    detailTitle.value = null
  }

  return {
    pageTitle,
    detailTitle,
    fullTitle,
    setPageTitle,
    setDetailTitle,
    resetTitles,
  }
})
