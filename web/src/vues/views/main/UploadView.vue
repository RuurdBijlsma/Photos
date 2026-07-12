<script setup lang="ts">
import * as tus from 'tus-js-client'

async function upload(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return

  const file = target.files[0]

  const upload = new tus.Upload(file, {
    endpoint: 'http://localhost:9475/files',
    retryDelays: [0, 3000, 5000, 10000, 20000],
    chunkSize: 50 * 1024 * 1024,
    metadata: {
      filename: file.name,
      filetype: file.type,
    },
    onBeforeRequest: function (req) {
      const xhr = req.getUnderlyingObject()
      if (xhr) {
        xhr.withCredentials = true
      }
    },
    // ... rest of your hooks
    onError: function (error) {
      console.log('Failed because: ' + error)
    },
    onProgress: function (bytesUploaded, bytesTotal) {
      const percentage = ((bytesUploaded / bytesTotal) * 100).toFixed(2)
      console.log(bytesUploaded, bytesTotal, percentage + '%')
    },
    onSuccess: function () {
      console.log('Upload finished for: %s. URL: %s', upload.file.name, upload.url)
    },
  })

  // Check if there are any previous uploads to resume
  upload.findPreviousUploads().then(function (previousUploads) {
    if (previousUploads.length) {
      upload.resumeFromPreviousUpload(previousUploads[0])
    }
    upload.start()
  })
}
</script>

<template>
  <div>
    <h1>Upload</h1>
    <input type="file" @change="upload" />
  </div>
</template>
