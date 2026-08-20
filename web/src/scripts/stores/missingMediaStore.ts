import MdiAlert from '~icons/mdi/alert'
import MdiDeleteForever from '~icons/mdi/delete-forever'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { StorageReviewItem } from '@/scripts/types/generated/timeline.ts'
import storageService from '@/scripts/services/storageService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import { useRefreshStore } from '@/scripts/stores/refreshStore.ts'

export const useMissingMediaStore = defineStore('missingMedia', () => {
  const snackbarStore = useSnackbarsStore()
  const dialogStore = useDialogStore()
  const selectionStore = useSelectionStore()
  const refreshStore = useRefreshStore()

  const missingItems = ref<StorageReviewItem[]>([])
  const totalSize = ref(0)
  const loading = ref(false)
  const pruning = ref(false)

  async function fetchMissing() {
    loading.value = true
    try {
      const response = await storageService.getMissingItems()
      missingItems.value = response.items
      totalSize.value = response.totalSize
    } catch (e) {
      snackbarStore.error("Can't fetch missing media items", e)
    } finally {
      loading.value = false
    }
  }

  function callForRefresh() {
    requestIdleCallback(() => {
      selectionStore.deselectAll(true).then()
      refreshStore.counter++
    })
  }

  function removeItem(id: string) {
    const item = missingItems.value.find((i) => i.id === id)
    if (item) {
      totalSize.value = Math.max(0, totalSize.value - item.sizeBytes)
      missingItems.value = missingItems.value.filter((i) => i.id !== id)
    }
  }

  async function pruneItems(ids: string[]) {
    if (ids.length === 0) return

    const confirmed = await dialogStore.confirm({
      title: 'Delete from Database?',
      color: 'error',
      icon: MdiDeleteForever,
      description: `Are you sure you want to permanently delete ${ids.length} missing item${ids.length === 1 ? '' : 's'} from the database? This cannot be undone.`,
      confirmText: 'Delete from Database',
    })

    if (!confirmed) return

    pruning.value = true
    try {
      const result = await storageService.pruneMissingItems(ids)
      snackbarStore.enqueue({
        message: `${result.prunedCount} item${result.prunedCount === 1 ? '' : 's'} removed from database`,
        icon: MdiDeleteForever,
      })
      const idSet = new Set(ids)
      missingItems.value = missingItems.value.filter((i) => !idSet.has(i.id))
      totalSize.value = missingItems.value.reduce((sum, item) => sum + item.sizeBytes, 0)
      callForRefresh()
    } catch (e) {
      snackbarStore.error('Failed to prune missing items', e)
    } finally {
      pruning.value = false
    }
  }

  async function pruneAll() {
    if (missingItems.value.length === 0) return

    let confirmed = await dialogStore.confirm({
      title: 'Prune All Missing Media?',
      color: 'error',
      icon: MdiDeleteForever,
      description: `Are you sure you want to delete all ${missingItems.value.length} missing media records from the database? Their thumbnails and metadata will be permanently removed.`,
      confirmText: 'Prune All',
    })

    if (!confirmed) return

    confirmed = await dialogStore.confirm({
      title: 'Are you sure?',
      color: 'error',
      icon: MdiAlert,
      description: `This will permanently delete records for all files currently missing from disk. This action <strong>cannot be undone</strong>.`,
      confirmText: 'Permanently Delete',
    })

    if (!confirmed) return

    pruning.value = true
    try {
      const result = await storageService.pruneMissingItems()
      snackbarStore.enqueue({
        message: `${result.prunedCount} item${result.prunedCount === 1 ? '' : 's'} pruned from database`,
        icon: MdiDeleteForever,
      })
      missingItems.value = []
      totalSize.value = 0
      callForRefresh()
    } catch (e) {
      snackbarStore.error('Failed to prune all missing items', e)
    } finally {
      pruning.value = false
    }
  }

  return {
    missingItems,
    totalSize,
    loading,
    pruning,
    fetchMissing,
    pruneItems,
    pruneAll,
    removeItem,
  }
})
