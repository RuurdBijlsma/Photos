import { defineStore } from 'pinia'
import { type Ref, ref } from 'vue'
import adminService from '@/scripts/services/adminService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import type { MediaSampleResponse, UnsupportedFilesResponse } from '@/scripts/types/api/admin.ts'
import { useDebounceFn } from '@vueuse/core'
import mediaItemService from '@/scripts/services/mediaItemService.ts'

export const usePickFolderStore = defineStore('pickFolder', () => {
  let N_SAMPLES = 8
  const unsupportedFilesLoading = ref(false)
  const mediaSampleLoading = ref(false)
  const listFolderLoading = ref(false)
  const makeFolderLoading = ref(false)

  const folderList: Ref<string[]> = ref([])
  const viewedFolder: Ref<string[]> = ref([])
  const mediaSamples: Ref<MediaSampleResponse | null> = ref(null)
  const unsupportedFiles: Ref<UnsupportedFilesResponse | null> = ref(null)
  const samples: Ref<{ imageUrl: string; relPath: string }[]> = ref(
    [...Array(N_SAMPLES)].map(() => ({ imageUrl: '', relPath: '' })),
  )
  const snackbarStore = useSnackbarsStore()

  const dbRefreshMediaSample = useDebounceFn(refreshMediaSample, 500)

  async function openFolder(folder: string) {
    viewedFolder.value.push(folder)
    await refreshFolders()
  }

  async function truncateViewed(index: number) {
    viewedFolder.value = viewedFolder.value.slice(0, index)
    await refreshFolders()
  }

  async function refreshFolders() {
    const folder = viewedFolder.value.join('/')
    try {
      const response = await adminService.getFolders(folder)
      folderList.value = response.data
    } catch (e) {
      await truncateViewed(viewedFolder.value.length - 2)
      snackbarStore.error(`Could not fetch sub-folders for: ${folder}`, e)
    }
    dbRefreshMediaSample()
  }

  async function getImageUrl(relative_path: string): Promise<string> {
    const url = await mediaBlobUrl(relative_path)
    return url ?? 'img/placeholder.svg'
  }

  async function refreshMediaSample() {
    const requestFolder = viewedFolder.value.join('/')

    mediaSampleLoading.value = true
    const response = await adminService.getMediaSample(requestFolder)
    mediaSampleLoading.value = false
    // Ignore result if the viewed folder has changed since making the request
    if (viewedFolder.value.join('/') !== requestFolder) return

    mediaSamples.value = response.data

    N_SAMPLES = mediaSamples.value.samples.length
    if (samples.value.length > N_SAMPLES) {
      samples.value = samples.value.slice(0, N_SAMPLES)
    } else if (samples.value.length < N_SAMPLES) {
      samples.value.concat(Array(N_SAMPLES - samples.value.length))
    }
    let j = 0
    for (let i = 0; i < N_SAMPLES; i++) {
      const relPath = mediaSamples.value.samples[i]
      if (relPath === undefined) continue
      getImageUrl(relPath).then((imageUrl) => {
        if (viewedFolder.value.join('/') !== requestFolder) return
        samples.value[j++] = { imageUrl, relPath }
      })
    }
  }

  async function refreshUnsupportedFiles() {
    const requestFolder = viewedFolder.value.join('/')

    unsupportedFilesLoading.value = true
    const response = await adminService.getUnsupportedFiles(requestFolder)
    unsupportedFilesLoading.value = false
    // Ignore result if the viewed folder has changed since making the request
    if (viewedFolder.value.join('/') !== requestFolder) return

    unsupportedFiles.value = response.data
  }

  async function makeFolder(folderName: string) {
    makeFolderLoading.value = true
    const baseFolder = viewedFolder.value.join('/')

    try {
      await adminService.makeFolder({ baseFolder, newName: folderName })
    } catch (e) {
      snackbarStore.error("Can't make folder", e)
    } finally {
      makeFolderLoading.value = false
    }

    refreshFolders().then()
  }

  async function mediaBlobUrl(relative_path: string): Promise<string | null> {
    try {
      const response = await mediaItemService.downloadMediaFile(relative_path)
      return URL.createObjectURL(response.data)
    } catch (error) {
      snackbarStore.error(`Failed to get full file url: ${relative_path}`, error)
    }
    return null
  }

  return {
    refreshFolders,
    openFolder,
    truncateViewed,
    makeFolder,
    mediaSamples,
    mediaSampleLoading,
    samples,
    refreshUnsupportedFiles,
    unsupportedFiles,
    unsupportedFilesLoading,
    listFolderLoading,
    viewedFolder,
    folderList,
  }
})
