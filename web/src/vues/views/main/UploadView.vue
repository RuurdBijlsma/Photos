<script setup lang="ts">
import * as tus from 'tus-js-client'
import uploadService from '@/scripts/services/uploadService.ts'
import apiClient from '@/scripts/services/api.ts'

async function upload(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return

  const file = target.files[0]

  let jwtToken = ''
  try {
    const { data } = await uploadService.getUploadJwt()
    jwtToken = data
  } catch (error) {
    console.error('Failed to retrieve upload JWT token:', error)
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
    onBeforeRequest: function (req) {
      const xhr = req.getUnderlyingObject()
      if (xhr) {
        xhr.withCredentials = true
      }
    },
    onError: function (error) {
      console.error('Upload failed:', error)
    },
    onProgress: function (bytesUploaded, bytesTotal) {
      console.log({
        bytesUploaded,
        bytesTotal,
        percentage: `${(bytesUploaded / bytesTotal) * 100}%`,
      })
    },
    onSuccess: function () {
      console.log('Upload finished for: %s. URL: %s', uploadInstance.file.name, uploadInstance.url)
    },
  })

  // Check if there are any previous uploads to resume
  uploadInstance.findPreviousUploads().then(function (previousUploads) {
    if (previousUploads.length) {
      uploadInstance.resumeFromPreviousUpload(previousUploads[0])
    }
    uploadInstance.start()
  })
}
</script>

<template>
  <div>
    <h1>Upload</h1>

    <div>
      <input type="file" @change="upload" />
    </div>
  </div>
</template>

<style scoped></style>
