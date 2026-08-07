import { mdiZipBoxOutline } from '@mdi/js'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { downloadBlob, filenameFromHeaders } from '@/scripts/utils.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'

export const useDownloadStore = defineStore('download', () => {
  const downloadingIds = ref<Set<string>>(new Set())
  const zipDownloading = ref(false)
  const anyDownloading = computed(() => zipDownloading.value || downloadingIds.value.size > 0)

  const snackbarStore = useSnackbarsStore()
  const mediaItemStore = useMediaItemStore()

  async function downloadItem(id: string) {
    const fullItem = mediaItemStore.mediaItems.get(id)
    if (downloadingIds.value.has(id)) return
    downloadingIds.value.add(id)
    try {
      const response = await mediaItemService.downloadMediaFileById(id)
      const filename = filenameFromHeaders(response.headers) ?? fullItem?.filename
      downloadBlob(response.data, filename)
    } catch (e) {
      snackbarStore.error('Could not download item', e)
    } finally {
      downloadingIds.value.delete(id)
    }
  }

  async function downloadItemsAsZip(ids: string[]) {
    if (zipDownloading.value) {
      snackbarStore.error("Can't download zip when a zip download is in progress")
      return
    }
    zipDownloading.value = true
    const snackId = snackbarStore.enqueue({
      message: `Preparing ZIP download for ${ids.length} items...`,
      icon: mdiZipBoxOutline,
      dismissable: false,
      loading: true,
      timeout: 0,
    })

    try {
      const response = await mediaItemService.downloadMediaZip(ids)

      const filename =
        filenameFromHeaders(response.headers) ??
        `photos_${new Date().toISOString().slice(0, 10)}.zip`
      downloadBlob(response.data, filename)
      snackbarStore.update(snackId, {
        dismissable: true,
        loading: false,
        message: 'Zipping complete',
        timeout: 5000,
      })
    } catch (e) {
      snackbarStore.remove(snackId)
      snackbarStore.error('Could not download ZIP archive', e)
    } finally {
      zipDownloading.value = false
    }
  }

  async function multiDownloadItems(ids: string[]) {
    if (ids.length > 5) {
      return await downloadItemsAsZip(ids)
    }
    for (const id of ids) {
      downloadItem(id).then()
    }
  }

  return {
    multiDownloadItems,
    downloadItem,

    downloadingIds,
    zipDownloading,
    anyDownloading,
  }
})
