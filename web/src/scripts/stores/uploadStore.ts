import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import * as tus from 'tus-js-client'
import uploadService from '@/scripts/services/uploadService.ts'
import { SERVER_BASE_URL } from '@/scripts/services/api.ts'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'

export interface UploadItem {
  id: string
  name: string
  size: number
  bytesUploaded: number
  status: 'pending' | 'uploading' | 'success' | 'failed' | 'stopped'
  error?: string
  tusUpload?: tus.Upload | null
}

export const useUploadStore = defineStore('upload', () => {
  const uploads = ref<UploadItem[]>([])
  const settings = useSettingStore()

  const fileMap = new Map<string, File>()

  let cachedJwt = ''
  let jwtFetchedAt = 0

  const activeCount = computed(() => uploads.value.filter((u) => u.status === 'uploading').length)
  const isUploading = computed(() =>
    uploads.value.some((u) => u.status === 'uploading' || u.status === 'pending'),
  )

  async function getValidJwt(): Promise<string> {
    const now = Date.now()
    if (cachedJwt && now - jwtFetchedAt < 150000) {
      return cachedJwt
    }
    const { data } = await uploadService.getUploadJwt()
    cachedJwt = data
    jwtFetchedAt = Date.now()
    return cachedJwt
  }

  function addFiles(files: FileList | File[]) {
    const itemsToAdd: UploadItem[] = []

    for (const file of Array.from(files)) {
      // Filter out 0-byte entries, which are highly likely to be directories or invalid uploads
      if (file.size === 0) {
        console.warn(`File "${file.name}" was skipped because it is empty (0 bytes).`)
        continue
      }

      // Check if a stopped or failed entry for this exact file is already present
      const existingItem = uploads.value.find(
        (u) =>
          u.name === file.name &&
          u.size === file.size &&
          (u.status === 'stopped' || u.status === 'failed'),
      )

      if (existingItem) {
        // Reuse and reset the entry instead of creating a duplicate
        existingItem.status = 'pending'
        existingItem.error = undefined
        fileMap.set(existingItem.id, file)
      } else {
        const id = `${file.name}-${file.size}-${Date.now()}-${Math.random().toString(36).substring(2, 7)}`
        itemsToAdd.push({
          id,
          name: file.name,
          size: file.size,
          bytesUploaded: 0,
          status: 'pending',
          tusUpload: null,
        })
        fileMap.set(id, file)
      }
    }

    if (itemsToAdd.length > 0) {
      uploads.value.push(...itemsToAdd)
    }
    processQueue()
  }

  function processQueue() {
    if (activeCount.value >= settings.uploadConcurrencyLimit) return

    const nextPending = uploads.value.find((u) => u.status === 'pending')
    if (!nextPending) return

    startUpload(nextPending)
    processQueue()
  }

  async function startUpload(item: UploadItem) {
    const file = fileMap.get(item.id)
    if (!file) {
      item.status = 'failed'
      item.error = 'File data is missing from reference mapping.'
      processQueue()
      return
    }

    item.status = 'uploading'

    let jwtToken = ''
    try {
      jwtToken = await getValidJwt()
    } catch (err: unknown) {
      item.status = 'failed'
      item.error = 'Unable to fetch upload token'
      console.error('[UploadStore] Error retrieving JWT:', err)
      processQueue()
      return
    }

    // todo: maybe broken with new /api/ prefix in backend
    const endpoint = `${SERVER_BASE_URL}/api/files`

    const uploadInstance = new tus.Upload(file, {
      endpoint,
      retryDelays: [0, 3000, 5000, 10000, 20000],
      chunkSize: 50 * 1024 * 1024,
      metadata: {
        filename: file.name,
        filetype: file.type,
        jwt: jwtToken,
      },
      onError: (error) => {
        if (item.status === 'stopped') return

        console.error(`Upload failed for ${file.name}:`, error)
        item.status = 'failed'
        item.error = error.message
        fileMap.delete(item.id)
        processQueue()
      },
      onProgress: (bytesUploaded) => {
        item.bytesUploaded = bytesUploaded
      },
      onSuccess: () => {
        item.status = 'success'
        item.bytesUploaded = item.size
        fileMap.delete(item.id)
        processQueue()
      },
    })

    item.tusUpload = uploadInstance

    uploadInstance
      .findPreviousUploads()
      .then((previousUploads) => {
        if (item.status !== 'uploading') return

        if (previousUploads.length) {
          uploadInstance.resumeFromPreviousUpload(previousUploads[0])
        }
        uploadInstance.start()
      })
      .catch((err) => {
        console.warn('Error seeking previous upload resume point:', err)
        if (item.status === 'uploading') {
          uploadInstance.start()
        }
      })
  }

  function stopUpload(id: string) {
    const item = uploads.value.find((u) => u.id === id)
    if (!item) return

    item.status = 'stopped'
    if (item.tusUpload) {
      try {
        item.tusUpload.abort()
      } catch (e) {
        console.warn('Failed to cleanly abort Tus process:', e)
      }
    }
    fileMap.delete(id)
    processQueue()
  }

  function removeUpload(id: string) {
    stopUpload(id)
    const idx = uploads.value.findIndex((u) => u.id === id)
    if (idx !== -1) {
      uploads.value.splice(idx, 1)
    }
  }

  function clearCompleted() {
    uploads.value = uploads.value.filter((u) => {
      if (u.status === 'success' || u.status === 'failed' || u.status === 'stopped') {
        fileMap.delete(u.id)
        return false
      }
      return true
    })
  }

  function abortAll() {
    uploads.value.forEach((u) => {
      if (u.status === 'uploading' || u.status === 'pending') {
        stopUpload(u.id)
      }
    })
  }

  return {
    uploads,
    activeCount,
    isUploading,
    addFiles,
    stopUpload,
    removeUpload,
    clearCompleted,
    abortAll,
  }
})
