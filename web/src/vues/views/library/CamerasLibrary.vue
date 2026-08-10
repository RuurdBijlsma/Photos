<script setup lang="ts">
import MdiCamera from '~icons/mdi/camera'
import MdiChevronDown from '~icons/mdi/chevron-down'
import MdiSortAlphabeticalDescendingVariant from '~icons/mdi/sort-alphabetical-descending-variant'
import MdiSortAlphabeticalAscendingVariant from '~icons/mdi/sort-alphabetical-ascending-variant'
import MdiSortNumericAscending from '~icons/mdi/sort-numeric-ascending'
import MdiSortNumericDescending from '~icons/mdi/sort-numeric-descending'
import { onMounted, computed } from 'vue'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import GlowThumbnail from '@/vues/components/ui/GlowThumbnail.vue'
import { useCameraStore } from '@/scripts/stores/cameraStore.ts'
import { useStorage } from '@vueuse/core'
import type { CameraInfo } from '@/scripts/types/generated/timeline.ts'
import { useRefreshFunction } from '@/scripts/composables/useRefreshFunction.ts'
import { useDelayedBoolean } from '@/scripts/composables/useDelayedBoolean.ts'

const cameraStore = useCameraStore()

// Sorting State
const currentSortField = useStorage<'make' | 'model' | 'photoCount'>(
  'cameraLibrarySortField',
  'photoCount',
)
const currentSortDirection = useStorage<'asc' | 'desc'>('cameraLibrarySortDirection', 'desc')

// Sorting Fields
const sortFields = [
  { title: 'Brand', field: 'make' },
  { title: 'Model', field: 'model' },
  { title: 'Photo count', field: 'photoCount' },
]

const currentSortFieldTitle = computed(() => {
  return sortFields.find((f) => f.field === currentSortField.value)?.title || 'Sort'
})

// Dynamically select the correct icon depending on the field and direction
const sortDirectionIcon = computed(() => {
  if (currentSortField.value === 'make' || currentSortField.value === 'model') {
    return currentSortDirection.value === 'asc'
      ? MdiSortAlphabeticalAscendingVariant
      : MdiSortAlphabeticalDescendingVariant
  }
  return currentSortDirection.value === 'asc' ? MdiSortNumericAscending : MdiSortNumericDescending
})

const sortDirectionTooltip = computed(() => {
  if (currentSortField.value === 'make' || currentSortField.value === 'model') {
    return currentSortDirection.value === 'asc' ? 'A-Z' : 'Z-A'
  }
  return currentSortDirection.value === 'asc' ? 'Fewest first' : 'Most first'
})

// Client-side sorting for the cameras array
const sortedCameras = computed(() => {
  const list = [...cameraStore.cameras]
  const field = currentSortField.value
  const dir = currentSortDirection.value === 'asc' ? 1 : -1

  return list.sort((a, b) => {
    const valA = a[field]
    const valB = b[field]

    if (typeof valA === 'string' && typeof valB === 'string') {
      return valA.localeCompare(valB) * dir
    }
    if (typeof valA === 'number' && typeof valB === 'number') {
      return (valA - valB) * dir
    }
    return 0
  })
})

const flickerLoad = useDelayedBoolean(() => cameraStore.camerasLoading, 150)
const showLoading = computed(() => flickerLoad.value && sortedCameras.value.length === 0)

function handleFieldChange(field: 'make' | 'model' | 'photoCount') {
  currentSortField.value = field
}

function toggleDirection() {
  currentSortDirection.value = currentSortDirection.value === 'asc' ? 'desc' : 'asc'
}

function getCameraThumbnailId(camera: CameraInfo) {
  // Use the direct thumbnail ID from the camera item if available;
  // otherwise, fall back to the first item from preloaded store media.
  return (
    camera.thumbnailId ||
    cameraStore.cameraMedia.get(camera.make + camera.model)?.items[0]?.id ||
    null
  )
}

onMounted(() => {
  cameraStore.fetchCameras()
})

useRefreshFunction(() => cameraStore.fetchCameras())
</script>

<template>
  <main-layout-container>
    <div class="library-container">
      <header class="library-header">
        <div class="header-left">
          <h1>Cameras</h1>
          <span class="album-count"
            >{{ sortedCameras.length }} camera{{ sortedCameras.length === 1 ? '' : 's' }}</span
          >
        </div>

        <div class="header-actions d-flex align-center">
          <v-menu location="bottom end">
            <template v-slot:activator="{ props }">
              <v-btn
                variant="text"
                color="primary"
                v-bind="props"
                rounded="xl"
                :append-icon="MdiChevronDown"
                class="text-none sort-text"
              >
                {{ currentSortFieldTitle }}
              </v-btn>
            </template>
            <v-list color="primary" density="compact">
              <v-list-item
                v-for="(option, index) in sortFields"
                :key="index"
                :title="option.title"
                :active="currentSortField === option.field"
                @click="handleFieldChange(option.field as 'make' | 'model' | 'photoCount')"
              />
            </v-list>
          </v-menu>

          <!-- Direction Toggle -->
          <v-btn
            variant="text"
            color="primary"
            class="sort-direction-button"
            :icon="sortDirectionIcon"
            @click="toggleDirection"
            v-tooltip="{
              location: 'top',
              text: sortDirectionTooltip,
            }"
          />
        </div>
      </header>

      <!-- Loading State -->
      <div v-if="showLoading" class="loading-state">
        <v-progress-circular indeterminate color="primary" size="48" />
      </div>

      <!-- Empty State -->
      <div
        v-else-if="!cameraStore.camerasLoading && sortedCameras.length === 0"
        class="empty-state"
      >
        <v-icon :icon="MdiCamera" size="100" class="mb-4 opacity-20" />
        <h2>No cameras found</h2>
        <p>Once you import media with camera metadata, they will appear here.</p>
      </div>

      <!-- Grid Layout -->
      <div v-else class="album-grid">
        <router-link
          v-for="camera in sortedCameras"
          :key="camera.make + camera.model"
          :to="`/camera/${encodeURIComponent(camera.make)}/${encodeURIComponent(camera.model)}`"
          class="album-card"
          @mouseenter="cameraStore.fetchCameraMedia(camera.make, camera.model)"
        >
          <div class="album-image">
            <!-- Icon placeholder fallback if no thumbnail is preloaded yet -->
            <div class="camera-thumbnail-placeholder" v-if="!getCameraThumbnailId(camera)">
              <v-icon :icon="MdiCamera" size="50" color="on-surface-variant" />
            </div>
            <glow-thumbnail
              v-else
              class="album-glow-image"
              :media-item-id="getCameraThumbnailId(camera)!"
              :height="200"
              :width="200"
              border-radius="20px"
              :strength="0.7"
            />
          </div>

          <div class="album-info">
            <h3
              v-tooltip="{
                location: 'top',
                text: camera.model || 'Unknown Model',
                disabled: (camera.model || 'Unknown Model').length <= 19,
              }"
              class="album-name text-truncate"
            >
              {{ camera.model || 'Unknown Model' }}
            </h3>
            <p class="album-meta">
              <span>{{ camera.make || 'Unknown Brand' }}</span>
              •
              <span>
                {{ camera.photoCount.toLocaleString() }} item{{
                  camera.photoCount === 1 ? '' : 's'
                }}
              </span>
            </p>
          </div>
        </router-link>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.library-container {
  padding: 20px 20px;
}

.library-header {
  padding: 0 10px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 30px;
}

.header-left h1 {
  margin: 0;
  font-size: 2.5rem;
  font-weight: 600;
  line-height: 1.2;
}

.album-count {
  font-size: 0.9rem;
  font-weight: 400;
  color: rgb(var(--v-theme-on-surface-variant));
}

.loading-state {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 100px 0;
}

.album-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 30px;
  justify-items: center;
}

.album-image {
  position: relative;
  height: 200px;
  width: 200px;
}

.camera-thumbnail-placeholder {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgb(var(--v-theme-surface-container-high));
  border-radius: 20px;
}

.album-glow-image {
  top: 0;
  left: 0;
  position: absolute;
  width: 100%;
  height: 100%;
}

.album-card {
  text-decoration: none;
  color: inherit;
  transition: transform 0.2s ease;
}

.album-card:hover {
  transform: translateY(-5px) scale(1.01);
}

.album-info {
  margin-top: 12px;
  padding: 0 4px;
}

.album-name {
  font-size: 1rem;
  font-weight: 600;
  margin-bottom: 2px;
  max-width: 195px;
  margin-top: 0;
}

.album-meta {
  font-size: 0.85rem;
  opacity: 0.6;
  margin-top: 0;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 100px 0;
  text-align: center;
}

.empty-state h2 {
  opacity: 0.8;
}

.empty-state p {
  opacity: 0.6;
}

.album-card:hover :deep(.glow-image-container) {
  box-shadow: 0 10px 30px -10px rgba(var(--v-theme-primary), 0.3);
}
</style>
