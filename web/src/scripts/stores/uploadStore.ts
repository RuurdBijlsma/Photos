import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import * as tus from 'tus-js-client'
import uploadService from '@/scripts/services/uploadService.ts'
import apiClient from '@/scripts/services/api.ts'
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

  // Local, non-reactive mapping to store original File instances
  // keeping the reactive store lightweight and performant
  const fileMap = new Map<string, File>()

  // JWT cache properties
  let cachedJwt = ''
  let jwtFetchedAt = 0

  // Computed state getters
  const activeCount = computed(() => uploads.value.filter((u) => u.status === 'uploading').length)
  const isUploading = computed(() =>
    uploads.value.some((u) => u.status === 'uploading' || u.status === 'pending'),
  )

  /**
   * Retrieves a cached JWT token if valid (under 2.5 minutes old),
   * otherwise requests a fresh token from the auth system.
   */
  async function getValidJwt(): Promise<string> {
    const now = Date.now()
    // JWT expires in 3 minutes. Request a new one if older than 150 seconds.
    if (cachedJwt && now - jwtFetchedAt < 150000) {
      return cachedJwt
    }
    const { data } = await uploadService.getUploadJwt()
    cachedJwt = data
    jwtFetchedAt = Date.now()
    return cachedJwt
  }

  /**
   * Appends files to the upload queue and begins processing
   */
  function addFiles(files: FileList | File[]) {
    const itemsToAdd: UploadItem[] = []

    for (const file of Array.from(files)) {
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

    uploads.value.push(...itemsToAdd)
    processQueue()
  }

  /**
   * Monitors active limits and kicks off pending uploads
   */
  function processQueue() {
    if (activeCount.value >= settings.uploadConcurrencyLimit) return

    const nextPending = uploads.value.find((u) => u.status === 'pending')
    if (!nextPending) return

    startUpload(nextPending)
    // Recurse to fill the remaining slots up to the concurrency limit
    processQueue()
  }

  /**
   * Initiates the Tus upload protocol for a specific item
   */
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
    } catch (err: any) {
      item.status = 'failed'
      item.error = 'Unable to fetch upload token'
      console.error('[UploadStore] Error retrieving JWT:', err)
      processQueue()
      return
    }

    const endpoint = `${apiClient.defaults.baseURL}/files`

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
        // Prevent updates if the operation was manually canceled
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

    // Query server for resume targets, then initiate
    uploadInstance
      .findPreviousUploads()
      .then((previousUploads) => {
        if (item.status !== 'uploading') return // Ensure user hasn't canceled during query

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

  /**
   * Stops an active upload, freeing up a worker spot in the queue
   */
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

  /**
   * Removes an item from the queue entirely
   */
  function removeUpload(id: string) {
    stopUpload(id)
    const idx = uploads.value.findIndex((u) => u.id === id)
    if (idx !== -1) {
      uploads.value.splice(idx, 1)
    }
  }

  /**
   * Clears out completed, canceled, and failed items
   */
  function clearCompleted() {
    uploads.value = uploads.value.filter((u) => {
      if (u.status === 'success' || u.status === 'failed' || u.status === 'stopped') {
        fileMap.delete(u.id)
        return false
      }
      return true
    })
  }

  /**
   * Cancels all remaining queued or active operations
   */
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
