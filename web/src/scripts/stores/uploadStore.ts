import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import uploadService from '@/scripts/services/uploadService.ts'
import * as tus from 'tus-js-client'
import apiClient from '@/scripts/services/api.ts'

export interface UploadItem {
  id: string
  file: File
  progress: number
  status: 'idle' | 'uploading' | 'completed' | 'failed'
  error?: string
  tusUpload?: tus.Upload
}

export const useUploadStore = defineStore('upload', () => {
  const snackbarStore = useSnackbarsStore()
  const uploadQueue = ref<UploadItem[]>([])

  function clearQueue() {
    uploadQueue.value = []
  }

  function addFiles(files: File[]) {
    const items: UploadItem[] = files.map((file) => ({
      id: Math.random().toString(36).substring(2, 9),
      file,
      progress: 0,
      status: 'idle',
    }))
    uploadQueue.value.push(...items)
    // Automatically start uploading idle items
    processQueue()
  }

  function processQueue() {
    const activeUploads = uploadQueue.value.filter((item) => item.status === 'uploading').length
    // Allow up to 2 concurrent uploads
    if (activeUploads >= 2) return

    const nextItem = uploadQueue.value.find((item) => item.status === 'idle')
    if (!nextItem) return

    startUpload(nextItem)
    processQueue() // Check if we can start another one
  }

  function startUpload(item: UploadItem) {
    item.status = 'uploading'

    const baseUrl = apiClient.defaults.baseURL || window.location.origin
    const endpoint = new URL('/files', baseUrl).href // <--- Update here to match backend

    const upload = new tus.Upload(item.file, {
      endpoint,
      retryDelays: [0, 1000, 3000, 5000],
      metadata: {
        filename: item.file.name,
        filetype: item.file.type,
      },
      onBeforeRequest(req) {
        const xhr = req.getUnderlyingObject()
        if (xhr) {
          xhr.withCredentials = true
        }
      },
      onError(error) {
        console.error('TUS upload failed:', error)
        item.status = 'failed'
        item.error = error.message
        snackbarStore.error(`Upload failed for ${item.file.name}`)
        processQueue()
      },
      onProgress(bytesUploaded, bytesTotal) {
        item.progress = Math.round((bytesUploaded / bytesTotal) * 100)
      },
      async onSuccess() {
        try {
          if (!upload.url) {
            throw new Error('Upload URL is missing')
          }
          const urlParts = upload.url.split('/')
          const uploadId = urlParts[urlParts.length - 1]!

          item.progress = 100

          // Let the backend know the upload is done so it moves and ingests the file
          await uploadService.notifyComplete(uploadId, item.file.name)

          item.status = 'completed'
          snackbarStore.success(`Successfully uploaded and queued ${item.file.name} for ingestion`)
        } catch (err: any) {
          console.error('Complete notification failed:', err)
          item.status = 'failed'
          item.error = err.message || 'Notification failed'
          snackbarStore.error(`Failed to process upload for ${item.file.name}`)
        } finally {
          processQueue()
        }
      },
    })

    item.tusUpload = upload
    upload.start()
  }

  function pauseUpload(item: UploadItem) {
    if (item.tusUpload && item.status === 'uploading') {
      item.tusUpload.abort()
      item.status = 'idle'
      processQueue()
    }
  }

  function resumeUpload(item: UploadItem) {
    if (item.status === 'idle' || item.status === 'failed') {
      item.status = 'idle'
      processQueue()
    }
  }

  return {
    uploadQueue,
    addFiles,
    clearQueue,
    pauseUpload,
    resumeUpload,
  }
})
