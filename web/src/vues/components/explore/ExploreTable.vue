<script setup lang="ts">
import MdiImageFilterHdr from '~icons/mdi/image-filter-hdr'
import MdiImageOutline from '~icons/mdi/image-outline'
import MdiSnowflake from '~icons/mdi/snowflake'
import MdiThermometer from '~icons/mdi/thermometer'
import MdiWaterPercent from '~icons/mdi/water-percent'
import MdiWeatherPouring from '~icons/mdi/weather-pouring'
import MdiWeatherWindy from '~icons/mdi/weather-windy'
import { watch } from 'vue'
import { useRouter } from 'vue-router'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

const exploreStore = useExploreStore()
const viewPhotoStore = useViewPhotoStore()
const router = useRouter()

// Simplified column layout
const headers = [
  { title: 'Preview', key: 'thumbnail', sortable: false, width: '80px' },
  { title: 'Size', key: 'sizeBytes', sortable: true },
  { title: 'Temp', key: 'temperature', sortable: true },
  { title: 'Wind', key: 'windSpeed', sortable: true },
  { title: 'Humidity', key: 'relative_humidity', sortable: true },
  { title: 'Downfall', key: 'precipitation', sortable: true },
  { title: 'Snow', key: 'snow', sortable: true },
  { title: 'ISO', key: 'iso', sortable: true },
  { title: 'Shutter', key: 'exposureTime', sortable: true },
  { title: 'Focal', key: 'focalLength', sortable: true },
  { title: 'Altitude', key: 'altitude', sortable: true },
]

// Keep the global photo viewer synchronized with current pagination and sorting
watch(
  () => exploreStore.items,
  (newItems) => {
    const ids = newItems.map((item) => item.id)
    viewPhotoStore.ids = ids

    const metadataMap = new Map()
    newItems.forEach((item) => {
      metadataMap.set(item.id, {
        id: item.id,
        isVideo: item.isVideo,
        hasThumbnails: item.hasThumbnails,
        durationMs: item.durationMs,
        takenAtLocal: item.takenAtLocal,
        ratio: 1,
      })
    })
    viewPhotoStore.idsMetadata = metadataMap
    viewPhotoStore.viewLink = '/explore/view/'
  },
  { immediate: true },
)

// Triggered on page size, pagination, or sorting changes
async function loadTableData(options: {
  page: number
  itemsPerPage: number
  sortBy: { key: string; order: 'asc' | 'desc' }[]
}) {
  exploreStore.page = options.page
  exploreStore.itemsPerPage = options.itemsPerPage
  exploreStore.sortBy = options.sortBy || []
  await exploreStore.fetchExploreTable()
}

// Opens native full-screen Lightbox on click
function onRowClick(event: PointerEvent, row: { item: { id: string } }) {
  const target = event.target as HTMLElement
  if (target.closest('button') || target.closest('.v-btn')) return

  if (row?.item) {
    router.push(`/explore/view/${row.item.id}`)
  }
}

function formatShutterSpeed(seconds: number | null): string {
  if (seconds === null || seconds === undefined) return '-'
  if (seconds >= 1) {
    return `${seconds.toFixed(1)}s`
  }
  const fraction = Math.round(1 / seconds)
  return `1/${fraction}s`
}

function formatFocalLength(mm: number | null): string {
  if (mm === null || mm === undefined) return '-'
  return `${Math.round(mm)}mm`
}

function formatCoords(lat: number | null, lon: number | null): string {
  if (lat === null || lon === null) return '-'
  return `${lat.toFixed(4)}, ${lon.toFixed(4)}`
}
</script>

<template>
  <v-card class="explore-table-card" flat>
    <!-- Styled Header to match ExploreHistograms cards -->
    <div class="card-header">
      <div class="header-texts">
        <h3 class="card-title">Media Catalog</h3>
        <p class="card-subtitle">Browse through metadata parameters and local weather attributes</p>
      </div>
    </div>

    <div class="table-container">
      <v-data-table-server
        v-model:items-per-page="exploreStore.itemsPerPage"
        v-model:page="exploreStore.page"
        :headers="headers"
        :items="exploreStore.items"
        :items-length="exploreStore.totalCount"
        :loading="exploreStore.isTableLoading"
        item-value="id"
        hover
        class="explore-server-table"
        @update:options="loadTableData"
        @click:row="onRowClick"
      >
        <!-- Thumbnail Column -->
        <template #[`item.thumbnail`]="{ item }">
          <div class="thumbnail-wrapper">
            <thumbnail-img v-if="item.id" :media-item-id="item.id" class="table-thumbnail" cover />
            <v-icon v-else size="small" color="primary" :icon="MdiImageOutline"></v-icon>
          </div>
        </template>

        <!-- Size Column -->
        <template #[`item.sizeBytes`]="{ item }">
          <span class="monospace-text text-on-surface">
            {{ item.sizeBytes !== null ? prettyBytes(item.sizeBytes) : '-' }}
          </span>
        </template>

        <!-- Weather: Temp Column -->
        <template #[`item.temperature`]="{ item }">
          <div v-if="item.temperature !== null" class="d-flex align-center">
            <v-icon :icon="MdiThermometer" size="small" color="orange" class="mr-1" />
            <span class="val-text">{{ item.temperature.toFixed(1) }}°C</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Weather: Wind Speed Column -->
        <template #[`item.windSpeed`]="{ item }">
          <div v-if="item.windSpeed !== null" class="d-flex align-center">
            <v-icon :icon="MdiWeatherWindy" size="small" color="blue-lighten-2" class="mr-1" />
            <span class="val-text">{{ item.windSpeed.toFixed(1) }} km/h</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Weather: Relative Humidity Column -->
        <template #[`item.relative_humidity`]="{ item }">
          <div v-if="item.relativeHumidity !== null" class="d-flex align-center">
            <v-icon :icon="MdiWaterPercent" size="small" color="blue-lighten-2" class="mr-1" />
            <span class="val-text">{{ item.relativeHumidity.toFixed(1) }}%</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Weather: Precipitation Column -->
        <template #[`item.precipitation`]="{ item }">
          <div v-if="item.precipitation !== null" class="d-flex align-center">
            <v-icon :icon="MdiWeatherPouring" size="small" color="blue-lighten-2" class="mr-1" />
            <span class="val-text">{{ item.precipitation.toFixed(1) }} mm</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Weather: Snow Column -->
        <template #[`item.snow`]="{ item }">
          <div v-if="item.snow !== null" class="d-flex align-center">
            <v-icon :icon="MdiSnowflake" size="small" color="blue-lighten-2" class="mr-1" />
            <span class="val-text">{{ item.snow.toFixed(1) }} mm</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Camera Settings: ISO Column -->
        <template #[`item.iso`]="{ item }">
          <span v-if="item.iso !== null" class="monospace-text font-weight-medium val-text">{{
            item.iso
          }}</span>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Camera Settings: Shutter Speed Column -->
        <template #[`item.exposureTime`]="{ item }">
          <span class="monospace-text val-text">{{ formatShutterSpeed(item.exposureTime) }}</span>
        </template>

        <!-- Camera Settings: Focal Length Column -->
        <template #[`item.focalLength`]="{ item }">
          <span class="monospace-text val-text">{{ formatFocalLength(item.focalLength) }}</span>
        </template>

        <!-- GPS: Altitude Column -->
        <template #[`item.altitude`]="{ item }">
          <div
            v-if="item.altitude !== null"
            class="d-flex align-center"
            :title="formatCoords(item.latitude, item.longitude)"
          >
            <v-icon :icon="MdiImageFilterHdr" size="small" color="teal" class="mr-1" />
            <span class="val-text">{{ Math.round(item.altitude) }}m</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>
      </v-data-table-server>
    </div>
  </v-card>
</template>

<style scoped>
.explore-table-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  padding: 24px;
  border: none !important;
  overflow: hidden;
}

/* Card Header styles mimicking ExploreHistograms.vue */
.card-header {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 24px;
}

.card-icon {
  color: rgb(var(--v-theme-primary));
  font-size: 28px;
}

.header-texts {
  display: flex;
  flex-direction: column;
}

.card-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.card-subtitle {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.table-container {
  margin-top: 8px;
}

.explore-server-table {
  background: transparent !important;
}

/* Add custom transition and hover matching other views */
.explore-server-table :deep(tbody tr) {
  cursor: pointer;
  transition: background-color 0.18s ease;
}

.explore-server-table :deep(tbody tr:hover) {
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
}

.thumbnail-wrapper {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(var(--v-border-color), 0.1);
  background-color: rgba(var(--v-theme-on-surface), 0.03);
}

.table-thumbnail {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.monospace-text {
  font-family: monospace;
  font-size: 0.85rem;
}

.val-text {
  color: rgb(var(--v-theme-on-surface));
}

.text-disabled {
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.5;
}
</style>
