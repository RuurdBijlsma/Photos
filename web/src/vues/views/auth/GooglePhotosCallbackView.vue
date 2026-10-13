<script setup lang="ts">
import { onMounted, ref } from 'vue'

const message = ref('Connecting to Google Photos...')
const isError = ref(false)

onMounted(() => {
  const params = new URLSearchParams(window.location.search)
  const code = params.get('code')
  const error = params.get('error')

  if (window.opener) {
    if (code) {
      window.opener.postMessage(
        { type: 'GOOGLE_PHOTOS_CODE', code },
        window.location.origin,
      )
      message.value = 'Authorized! Closing window...'
      window.close()
    } else if (error) {
      window.opener.postMessage(
        { type: 'GOOGLE_PHOTOS_ERROR', error },
        window.location.origin,
      )
      isError.value = true
      message.value = `Authentication failed: ${error}`
      setTimeout(() => window.close(), 1500)
    } else {
      isError.value = true
      message.value = 'No authorization code found in callback URL.'
    }
  } else {
    isError.value = true
    message.value = 'Parent window not found. You can close this tab.'
  }
})
</script>

<template>
  <div class="callback-container">
    <v-progress-circular
      v-if="!isError"
      indeterminate
      color="primary"
      size="48"
      class="mb-4"
    />
    <v-icon
      v-else
      size="48"
      color="error"
      class="mb-4"
      icon="$error"
    />
    <p class="text-body-1">{{ message }}</p>
  </div>
</template>

<style scoped>
.callback-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  width: 100vw;
  background-color: #101010;
  color: #fff;
  text-align: center;
  padding: 24px;
}
</style>
