<script setup lang="ts">
import MdiGoogleCirclesExtended from '~icons/mdi/google-circles-extended'
import MdiCheckCircleOutline from '~icons/mdi/check-circle-outline'
import MdiAlertCircleOutline from '~icons/mdi/alert-circle-outline'
import MdiEye from '~icons/mdi/eye'
import MdiEyeOff from '~icons/mdi/eye-off'
import { onMounted, ref } from 'vue'
import googlePhotosService from '@/scripts/services/googlePhotosService'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore'
import type { GooglePhotosConfigStatus } from '@/scripts/types/api/googlePhotos'
import { SERVER_BASE_URL } from '@/scripts/services/api.ts'

const snackbars = useSnackbarsStore()

const status = ref<GooglePhotosConfigStatus | null>(null)
const loading = ref(true)
const saving = ref(false)
const showSecret = ref(false)

const clientId = ref('')
const clientSecret = ref('')

async function fetchStatus() {
  loading.value = true
  try {
    const { data } = await googlePhotosService.getConfigStatus()
    status.value = data
  } catch (e) {
    snackbars.error('Failed to load Google Photos config status', e)
  } finally {
    loading.value = false
  }
}

async function saveConfig() {
  if (!clientId.value.trim() || !clientSecret.value.trim()) {
    snackbars.error('Both Client ID and Client Secret are required.')
    return
  }
  saving.value = true
  try {
    await googlePhotosService.saveConfig({
      clientId: clientId.value.trim(),
      clientSecret: clientSecret.value.trim(),
    })
    snackbars.success('Google Photos credentials saved.')
    clientSecret.value = ''
    await fetchStatus()
  } catch (e) {
    snackbars.error('Failed to save Google Photos config', e)
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  fetchStatus()
})
</script>

<template>
  <div class="settings-card">
    <div class="card-header">
      <div class="card-title-group">
        <v-icon :icon="MdiGoogleCirclesExtended" color="primary" class="mr-3" size="22" />
        <span class="card-title">Google Photos Import</span>
      </div>
      <v-chip
        v-if="status?.configured"
        color="success"
        variant="tonal"
        size="small"
        :prepend-icon="MdiCheckCircleOutline"
      >
        Configured
      </v-chip>
      <v-chip
        v-else-if="status && !status.configured"
        color="warning"
        variant="tonal"
        size="small"
        :prepend-icon="MdiAlertCircleOutline"
      >
        Not Configured
      </v-chip>
    </div>

    <div class="card-body">
      <div v-if="loading" class="loading-state">
        <v-progress-circular indeterminate color="primary" size="32" />
      </div>

      <template v-else>
        <v-alert type="info" variant="tonal" rounded="lg" class="mb-5" density="compact">
          <div class="text-body-2">
            To enable Google Photos album import for your users:
            <ol class="mt-1 ml-3">
              <li>Create a project in <strong>Google Cloud Console</strong></li>
              <li>Enable the <strong>Google Photos Picker API</strong></li>
              <li>Create OAuth 2.0 credentials (Web Application)</li>
              <li>
                Add <code>{{ `${SERVER_BASE_URL}/google-photos-callback` }}</code> as an authorized
                redirect URI
              </li>
              <li>
                Set the OAuth consent screen to <strong>External / Testing</strong> and add your
                Google account as a test user
              </li>
            </ol>
          </div>
        </v-alert>

        <div v-if="status?.configured" class="mb-4">
          <span class="section-subtitle">Current Client ID</span>
          <p class="text-body-2 mt-1">{{ status.clientId }}</p>
        </div>

        <v-text-field
          v-model="clientId"
          name="ClientID"
          label="Client ID"
          variant="outlined"
          rounded="xl"
          density="comfortable"
          class="mb-3"
          :placeholder="status?.configured ? 'Enter new Client ID to replace…' : ''"
          persistent-placeholder
        />

        <v-text-field
          v-model="clientSecret"
          label="Client Secret"
          name="ClientSecret"
          variant="outlined"
          rounded="xl"
          density="comfortable"
          class="mb-4"
          :type="showSecret ? 'text' : 'password'"
          :append-inner-icon="showSecret ? MdiEyeOff : MdiEye"
          @click:append-inner="showSecret = !showSecret"
          :placeholder="status?.configured ? 'Enter new Client Secret to replace…' : ''"
          persistent-placeholder
        />

        <v-btn color="primary" variant="tonal" rounded :loading="saving" @click="saveConfig">
          {{ status?.configured ? 'Update Credentials' : 'Save Credentials' }}
        </v-btn>
      </template>
    </div>
  </div>
</template>

<style scoped>
.settings-card {
  background-color: rgb(var(--v-theme-surface-container-low));
  border-radius: 24px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.card-header {
  background-color: rgb(var(--v-theme-surface-container-high));
  padding: 16px 24px;
  border-top-left-radius: 24px;
  border-top-right-radius: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.card-title-group {
  display: flex;
  align-items: center;
}

.card-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.card-body {
  padding: 24px;
}

.section-subtitle {
  font-size: 0.725rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  color: rgb(var(--v-theme-secondary));
  text-transform: uppercase;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 32px 0;
}
</style>
