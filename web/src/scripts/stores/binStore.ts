import { mdiAlert, mdiDelete, mdiDeleteForever, mdiRestore } from '@mdi/js'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import binService from '@/scripts/services/binService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useRefreshStore } from '@/scripts/stores/refreshStore.ts'
import { usePeopleStore } from '@/scripts/stores/peopleStore.ts'
import { useObjStorage } from '@/scripts/utils.ts'

export const useBinStore = defineStore('bin', () => {
  const snackbarStore = useSnackbarsStore()
  const dialogStore = useDialogStore()
  const selectionStore = useSelectionStore()
  const albumStore = useAlbumStore()
  const peopleStore = usePeopleStore()
  const refreshStore = useRefreshStore()

  const binItems = useObjStorage<SimpleTimelineItem[]>('userBinItems', [])
  const binLoading = ref(false)

  async function fetchBin() {
    binLoading.value = true
    try {
      const response = await binService.getBinTimeline()
      binItems.value = response.items
    } catch (e) {
      snackbarStore.error("Can't fetch bin items", e)
    } finally {
      binLoading.value = false
    }
  }

  function callForRefresh() {
    requestIdleCallback(() => {
      albumStore.fetchUserAlbums().then()
      peopleStore.fetchPeople().then()
      selectionStore.deselectAll(true).then()
      refreshStore.counter++
    })
  }

  const softDeleteLoading = ref(false)
  async function softDeleteItems(ids: string[]) {
    softDeleteLoading.value = true
    try {
      await binService.softDelete(ids)
      snackbarStore.enqueue({
        message: `${ids.length} item${ids.length === 1 ? '' : 's'} moved to bin`,
        icon: mdiDelete,
        action: {
          label: 'Undo',
          onClick: () => restoreItems(ids),
        },
      })
      callForRefresh()
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
        icon: mdiRestore,
      })
      callForRefresh()
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
      icon: mdiDelete,
      description: `Are you sure you want to permanently delete ${ids.length} item${ids.length === 1 ? '' : 's'}? This action cannot be undone and will delete the files from your storage.`,
      confirmText: 'Delete Permanently',
    })

    if (!confirmed) return
    confirmed = await dialogStore.confirm({
      title: 'Are you sure??',
      color: 'error',
      icon: mdiAlert,
      description: `Are you super sure you want to permanently delete ${ids.length} item${ids.length === 1 ? '' : 's'}? This action <strong>cannot be undone</strong> and will delete the files from your storage.`,
      confirmText: 'Delete Permanently',
    })

    if (!confirmed) return
    hardDeleteLoading.value = true

    try {
      await binService.deletePermanently(ids)
      snackbarStore.enqueue({
        message: `${ids.length} item${ids.length === 1 ? '' : 's'} permanently deleted`,
        icon: mdiDeleteForever,
      })
      callForRefresh()
    } catch (e) {
      snackbarStore.error('Failed to permanently delete items', e)
    } finally {
      hardDeleteLoading.value = false
    }
  }

  return {
    binItems,
    binLoading,
    fetchBin,
    softDeleteItems,
    restoreItems,
    hardDeleteItems,
    hardDeleteLoading,
    softDeleteLoading,
    restoreLoading,
  }
})
