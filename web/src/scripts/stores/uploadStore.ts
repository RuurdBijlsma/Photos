import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import { useStorage } from '@vueuse/core'
import * as tus from 'tus-js-client'
import apiClient from '@/scripts/services/api.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import uploadService from '@/scripts/services/uploadService.ts'

// This interface represents the metadata stored on disk between sessions
export interface PersistedUpload {
  id: string
  filename: string
  size: number
  progress: number
  status: 'paused' | 'failed' | 'completed'
  uploadUrl?: string
}

// Runtime interface representing active transfers in memory
export interface UploadItem extends Omit<PersistedUpload, 'status'> {
  status: 'idle' | 'uploading' | 'paused' | 'failed' | 'completed'
  file?: File // Undefined on page refresh until re-associated
  tusUpload?: tus.Upload
  error?: string
}

export const useUploadStore = defineStore('upload', () => {
  const snackbarStore = useSnackbarsStore()

  // 1. Persisted storage sync (stores metadata only)
  const persistedQueue = useStorage<PersistedUpload[]>('ruurd-photos-upload-metadata', [])

  // 2. Memory queue (initialized with persisted metadata on reload)
  const uploadQueue = ref<UploadItem[]>(
    persistedQueue.value.map((item) => ({
      ...item,
      // Completed uploads remain completed; incomplete uploads are marked paused
      status: item.status === 'completed' ? 'completed' : 'paused',
      file: undefined,
    })),
  )

  // 3. Keep local storage in sync whenever our runtime queue updates
  watch(
    uploadQueue,
    (newQueue) => {
      persistedQueue.value = newQueue.map((item) => ({
        id: item.id,
        filename: item.filename,
        size: item.size,
        progress: item.progress,
        status: item.status === 'completed' ? 'completed' : 'paused',
        uploadUrl: item.uploadUrl,
      }))
    },
    { deep: true },
  )

  // 4. File picker/Drop handler with automatic re-association logic
  function addFiles(files: File[]) {
    for (const file of files) {
      // Find an incomplete item matching by name and exact size
      const matchingInterrupted = uploadQueue.value.find(
        (item) =>
          item.filename === file.name && item.size === file.size && item.status !== 'completed',
      )

      if (matchingInterrupted) {
        // Re-bind file handle to the existing metadata row
        matchingInterrupted.file = file
        matchingInterrupted.status = 'idle'
        console.log(`[Upload] File handle re-associated for: ${file.name}`)
      } else {
        // Create a new visual row
        const newId =
          typeof crypto.randomUUID === 'function'
            ? crypto.randomUUID()
            : Math.random().toString(36).substring(2, 15)

        uploadQueue.value.push({
          id: newId,
          filename: file.name,
          size: file.size,
          progress: 0,
          status: 'idle',
          file,
        })
      }
    }
    processQueue()
  }

  // 5. Triggering or resuming uploads
  function startUpload(item: UploadItem) {
    if (!item.file) {
      item.status = 'paused'
      console.warn(`[Upload] Attempted to start upload for ${item.filename} without a file handle.`)
      promptUserForFile(item)
      return
    }

    item.status = 'uploading'
    const baseUrl = apiClient.defaults.baseURL || window.location.origin
    const endpoint = new URL('/files', baseUrl).href

    const upload = new tus.Upload(item.file, {
      endpoint,
      uploadUrl: item.uploadUrl, // Resume existing TUS session if present
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
        // Log cleanly to avoid polluting UI console, then mark failed
        console.error(`[Upload] TUS streaming error for ${item.filename}:`, error)
        item.status = 'failed'
        item.error = error.message
        snackbarStore.error(`Upload failed for ${item.filename}: ${error.message}`)
        processQueue()
      },
      onProgress(bytesUploaded, bytesTotal) {
        item.progress = Math.round((bytesUploaded / bytesTotal) * 100)
      },
      async onSuccess() {
        try {
          if (!upload.url) {
            throw new Error('Upload reached 100% but returned an empty target URL.')
          }
          if (!item.file) {
            throw new Error('Associated file reference was lost before post-processing completed.')
          }

          const urlParts = upload.url.split('/')
          const uploadId = urlParts[urlParts.length - 1]!

          item.progress = 100

          console.log(
            `[Upload] File stream complete: ${item.filename}. Requesting backend move and ingest...`,
          )

          // Let the backend know the upload is done so it moves and ingests the file
          await uploadService.notifyComplete(uploadId, item.file.name)

          item.status = 'completed'
          snackbarStore.success(`Successfully uploaded and queued ${item.file.name} for ingestion`)
        } catch (err: any) {
          console.error(`[Upload] Ingestion post-processing failed for ${item.filename}:`, err)
          item.status = 'failed'
          item.error = err.message || 'Verification and ingestion failed'
          snackbarStore.error(`Failed to process upload for ${item.filename}`)
        } finally {
          processQueue()
        }
      },
    })

    item.tusUpload = upload

    // Resolve previous session URLs if they exist in client storage
    if (!item.uploadUrl) {
      upload
        .findPreviousUploads()
        .then((previous) => {
          if (previous.length > 0) {
            item.uploadUrl = previous[0].uploadUrl ?? undefined
            console.log(
              `[Upload] Resuming ${item.filename} from existing TUS offset: ${item.uploadUrl}`,
            )
          }
          upload.start()
        })
        .catch((err) => {
          console.warn('[Upload] Failed to query TUS cache, starting fresh:', err)
          upload.start()
        })
    } else {
      upload.start()
    }
  }

  // 6. Action: Pause Upload
  function pauseUpload(item: UploadItem) {
    if (item.tusUpload && item.status === 'uploading') {
      item.tusUpload.abort()
      item.status = 'paused'
      console.log(`[Upload] Manually paused: ${item.filename}`)
      processQueue()
    }
  }

  // 7. Action: Resume Upload
  function resumeUpload(item: UploadItem) {
    if (item.status === 'paused' || item.status === 'failed') {
      item.status = 'idle'
      console.log(`[Upload] Scheduled resume for: ${item.filename}`)
      processQueue()
    }
  }

  // Track which item is waiting for the user to manually browse/re-locate it
  const pendingFileReassociation = ref<UploadItem | null>(null)

  function promptUserForFile(item: UploadItem) {
    pendingFileReassociation.value = item
    snackbarStore.info(`Please re-select or drop "${item.filename}" to resume your upload.`)
  }

  function handleManualFileSelection(file: File) {
    const item = pendingFileReassociation.value
    if (item && item.filename === file.name && item.size === file.size) {
      item.file = file
      item.status = 'idle'
      pendingFileReassociation.value = null
      snackbarStore.success(`File handle re-associated successfully: ${file.name}`)
      processQueue()
    } else {
      snackbarStore.error(
        `Selected file does not match the expected name or size of "${item?.filename}"`,
      )
    }
  }

  // Safe queue cleanup (aborts any active network transfers before purging)
  function clearQueue() {
    console.log('[Upload] Purging upload queue. Aborting active transfers...')
    uploadQueue.value.forEach((item) => {
      if (item.tusUpload && item.status === 'uploading') {
        item.tusUpload.abort()
      }
    })
    uploadQueue.value = []
  }

  // Manages concurrent upload pool size (caps concurrent streams at 2)
  function processQueue() {
    const active = uploadQueue.value.filter((i) => i.status === 'uploading').length
    if (active >= 2) return

    const next = uploadQueue.value.find((i) => i.status === 'idle')
    if (!next) return

    startUpload(next)
    processQueue()
  }

  return {
    uploadQueue,
    pendingFileReassociation,
    addFiles,
    pauseUpload,
    resumeUpload,
    clearQueue,
    handleManualFileSelection,
  }
})
