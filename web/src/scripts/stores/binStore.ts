import { defineStore } from 'pinia'
import { ref, shallowRef } from 'vue'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import binService from '@/scripts/services/binService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import { useTimelineStore } from '@/scripts/stores/timeline/timelineStore.ts'
import { useRouter } from 'vue-router'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'

export const useBinStore = defineStore('bin', () => {
  const snackbarStore = useSnackbarsStore()
  const dialogStore = useDialogStore()
  const selectionStore = useSelectionStore()
  const timelineStore = useTimelineStore()
  const albumStore = useAlbumStore()
  const router = useRouter()

  const binItems = shallowRef<SimpleTimelineItem[]>([])
  const isLoading = ref(false)

  async function fetchBin() {
    isLoading.value = true
    try {
      const response = await binService.getBinTimeline()
      binItems.value = response.items
    } catch (e) {
      snackbarStore.error("Can't fetch bin items", e)
    } finally {
      isLoading.value = false
    }
  }

  const softDeleteLoading = ref(false)
  async function softDeleteItems(ids: string[]) {
    softDeleteLoading.value = true
    try {
      await binService.softDelete(ids)
      snackbarStore.enqueue({
        message: `${ids.length} item${ids.length === 1 ? '' : 's'} moved to bin`,
        icon: 'mdi-delete',
        action: {
          label: 'Bin',
          onClick: () => {
            router.push({ path: '/bin' })
          },
        },
      })
      requestIdleCallback(() => {
        fetchBin()
        albumStore.fetchUserAlbums()
        timelineStore.refresh()
        selectionStore.deselectAll(true)
      })
    } catch (e) {
      snackbarStore.error('Failed to move items to bin', e)
    } finally {
      softDeleteLoading.value = false
    }
  }

  const restoreLoading = ref(false)
  async function restoreItems(ids: string[]) {
    restoreLoading.value = true
    try {
      await binService.restore(ids)
      snackbarStore.enqueue({
        message: `${ids.length} item${ids.length === 1 ? '' : 's'} restored`,
        icon: 'mdi-restore',
      })
      requestIdleCallback(() => {
        fetchBin()
        albumStore.fetchUserAlbums()
        timelineStore.refresh()
        selectionStore.deselectAll(true)
      })
    } catch (e) {
      snackbarStore.error('Failed to restore items', e)
    } finally {
      restoreLoading.value = false
    }
  }

  const hardDeleteLoading = ref(false)
  async function hardDeleteItems(ids: string[]) {
    let confirmed = await dialogStore.confirm({
      title: 'Delete Permanently?',
      color: 'error',
      icon: 'mdi-delete',
      description: `Are you sure you want to permanently delete ${ids.length} item${ids.length === 1 ? '' : 's'}? This action cannot be undone and will delete the files from your storage.`,
      confirmText: 'Delete Permanently',
    })

    if (!confirmed) return
    confirmed = await dialogStore.confirm({
      title: 'Are you sure??',
      color: 'error',
      icon: 'mdi-alert',
      description: `Are you super sure you want to permanently delete ${ids.length} item${ids.length === 1 ? '' : 's'}? This action <strong>cannot be undone</strong> and will delete the files from your storage.`,
      confirmText: 'Delete Permanently',
    })

    if (!confirmed) return
    hardDeleteLoading.value = true

    try {
      await binService.deletePermanently(ids)
      snackbarStore.enqueue({
        message: `${ids.length} item${ids.length === 1 ? '' : 's'} permanently deleted`,
        icon: 'mdi-delete-forever',
      })
      requestIdleCallback(() => {
        fetchBin()
        albumStore.fetchUserAlbums()
        timelineStore.refresh()
        selectionStore.deselectAll()
      })
    } catch (e) {
      snackbarStore.error('Failed to permanently delete items', e)
    } finally {
      hardDeleteLoading.value = false
    }
  }

  return {
    binItems,
    isLoading,
    fetchBin,
    softDeleteItems,
    restoreItems,
    hardDeleteItems,
    hardDeleteLoading,
    softDeleteLoading,
    restoreLoading,
  }
})
