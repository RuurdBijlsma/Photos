<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import albumService from '@/scripts/services/albumService.ts'
import type { AlbumSummary } from '@/scripts/types/api/album.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { isAxiosError } from 'axios'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'

const route = useRoute()
const router = useRouter()
const snackbars = useSnackbarsStore()
const albumStore = useAlbumStore()

const token = computed(() => route.params.token as string)
const summary = ref<AlbumSummary | null>(null)
const loading = ref(true)
const error = ref<string | null>(null)
const importing = ref(false)

const localName = ref('')
const localDescription = ref('')

async function fetchSummary() {
  if (!token.value) return
  loading.value = true
  error.value = null
  try {
    const { data } = await albumService.checkInvite({ token: token.value })
    summary.value = data
    localName.value = data.name
    localDescription.value = data.description || ''
  } catch (e: unknown) {
    console.error('Failed to check invite', e)
    if (isAxiosError(e) && (e.response?.status === 401 || e.response?.status === 404)) {
      error.value = 'This invitation has expired or is invalid.'
    } else {
      error.value = 'Failed to load album preview. Please try again later.'
    }
  } finally {
    loading.value = false
  }
}

async function startImport() {
  if (!token.value) return
  importing.value = true
  try {
    const { data: newAlbum } = await albumService.acceptInvite({
      token: token.value,
      name: localName.value,
      description: localDescription.value,
    })

    await albumStore.fetchUserAlbums()
    snackbars.success('Import started! Photos will appear shortly.')

    await router.push({
      name: 'album-view',
      params: { albumId: newAlbum.id },
      query: { importing: 'true' },
    })
  } catch (e) {
    snackbars.error('Failed to start import', e)
    importing.value = false
  }
}

onMounted(() => {
  fetchSummary()
})
</script>

<template>
  <main-layout-container fit-to-content>
    <div class="import-container">
      <!-- Loading State -->
      <div v-if="loading" class="state-container">
        <v-progress-circular indeterminate color="primary" size="64" />
        <p class="mt-4 text-h6 opacity-70">Fetching invitation details...</p>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="state-container">
        <v-icon icon="mdi-alert-circle-outline" color="error" size="80" />
        <h2 class="text-h4 mt-4">Invalid Link</h2>
        <p class="opacity-70 mt-2">{{ error }}</p>
        <v-btn color="primary" variant="tonal" rounded="xl" class="mt-6" to="/"> Go Home </v-btn>
      </div>

      <!-- Content State -->
      <div v-else-if="summary" class="import-content">
        <div class="header-section mb-8">
          <v-icon icon="mdi-share-variant" color="primary" size="48" class="mb-4" />
          <h1 class="text-h3 font-weight-bold">Import Album</h1>
          <p class="text-subtitle-1 opacity-70">Someone shared a collection with you</p>
        </div>

        <v-row justify="center">
          <v-col cols="12" md="8" lg="6">
            <v-card color="surface-container-high" rounded="xl" class="pa-6" elevation="2">
              <div class="d-flex align-center mb-6">
                <v-avatar color="primary" variant="tonal" size="64" rounded="lg" class="mr-4">
                  <v-icon icon="mdi-image-multiple" size="32" />
                </v-avatar>
                <div>
                  <div class="text-h5 font-weight-bold">{{ summary.name }}</div>
                  <div class="text-body-1 opacity-60">
                    Contains {{ summary.relativePaths.length }} items
                  </div>
                </div>
              </div>

              <v-divider class="mb-6" />

              <div class="form-section">
                <p class="text-overline mb-4">Your Settings</p>
                <v-text-field
                  v-model="localName"
                  label="Album Name"
                  variant="outlined"
                  rounded="lg"
                  prepend-inner-icon="mdi-pencil"
                  class="mb-2"
                  persistent-placeholder
                />
                <v-textarea
                  v-model="localDescription"
                  label="Description (Optional)"
                  variant="outlined"
                  rounded="lg"
                  prepend-inner-icon="mdi-text"
                  rows="3"
                  auto-grow
                />
              </div>

              <v-card-actions class="pa-0 mt-6">
                <v-btn
                  color="primary"
                  size="x-large"
                  block
                  rounded="xl"
                  variant="flat"
                  class="import-btn"
                  :loading="importing"
                  @click="startImport"
                >
                  Confirm Import
                </v-btn>
              </v-card-actions>
            </v-card>

            <p class="mt-6 text-center text-body-2 opacity-50">
              This will create a local copy of the album in your library.
            </p>
          </v-col>
        </v-row>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.import-container {
  padding: 40px 20px;
}

.import-content {
  max-width: 1000px;
  margin: 0 auto;
}

.state-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}

.header-section {
  text-align: center;
}

.import-btn {
  font-weight: 700;
  letter-spacing: 0.5px;
  /* Matching the Glow effect in AlbumsLibrary */
  box-shadow: 0 10px 20px -10px rgba(var(--v-theme-primary), 0.5) !important;
}

.form-section :deep(.v-field__outline) {
  --v-field-border-opacity: 0.15;
}

/* Mobile adjustments */
@media (max-width: 600px) {
  .text-h3 {
    font-size: 2rem !important;
  }
}
</style>
