import { ref, type Ref } from 'vue'
import { defineStore } from 'pinia'
import type { ExploreMediaItem } from '@/scripts/types/api/explore.ts'
import exploreService from '@/scripts/services/exploreService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'

export const useExploreStore = defineStore('explore', () => {
  const snackbarStore = useSnackbarsStore()

  // --- STATE ---
  const items: Ref<ExploreMediaItem[]> = ref([])
  const totalCount = ref(0)
  const isTableLoading = ref(false)

  // Pagination & Datatable Parameters
  const page = ref(1)
  const itemsPerPage = ref(15)
  const sortBy: Ref<{ key: string; order: 'asc' | 'desc' }[]> = ref([])

  // --- ACTIONS ---
  async function fetchExploreTable() {
    isTableLoading.value = true
    try {
      // Map Pinia sortBy states into the "key:order" syntax expected by the backend
      const sortParams: string[] = []
      if (sortBy.value && sortBy.value.length > 0) {
        sortBy.value.forEach((s) => {
          sortParams.push(`${s.key}:${s.order}`)
        })
      }

      const response = await exploreService.getExploreTable({
        page: page.value,
        limit: itemsPerPage.value,
        sort: sortParams,
      })

      items.value = response.data.data
      totalCount.value = response.data.total
    } catch (error) {
      snackbarStore.error('Failed to load explore stats', error)
    } finally {
      isTableLoading.value = false
    }
  }

  function resetPagination() {
    page.value = 1
    items.value = []
    totalCount.value = 0
  }

  return {
    items,
    totalCount,
    isTableLoading,
    page,
    itemsPerPage,
    sortBy,
    fetchExploreTable,
    resetPagination,
  }
})
