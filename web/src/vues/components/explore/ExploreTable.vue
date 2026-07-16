<script setup lang="ts">
import { watch } from 'vue'
import { useRouter } from 'vue-router'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

const exploreStore = useExploreStore()
const viewPhotoStore = useViewPhotoStore()
const router = useRouter()

// Simplified column layout (removed Date Taken, Filename, and Aperture)
const headers = [
  { title: 'Preview', key: 'thumbnail', sortable: false, width: '80px' },
  { title: 'Size', key: 'sizeBytes', sortable: true },
  { title: 'Temp', key: 'temperature', sortable: true },
  { title: 'Wind', key: 'windSpeed', sortable: true },
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
  <v-card class="explore-table-card" flat border>
    <div class="table-header-row">
      <div class="header-title-group">
        <span class="header-title">Media Stats Explorer</span>
        <v-icon color="primary" class="ml-2">mdi-chart-scatter-plot</v-icon>
      </div>
      <v-spacer />
      <v-btn
        prepend-icon="mdi-refresh"
        variant="tonal"
        color="primary"
        rounded
        :loading="exploreStore.isTableLoading"
        @click="exploreStore.fetchExploreTable"
      >
        Refresh
      </v-btn>
    </div>

    <div class="table-body">
      <v-data-table-server
        v-model:items-per-page="exploreStore.itemsPerPage"
        v-model:page="exploreStore.page"
        :headers="headers"
        :items="exploreStore.items"
        :items-length="exploreStore.totalCount"
        :loading="exploreStore.isTableLoading"
        item-value="id"
        multi-sort
        hover
        class="explore-server-table"
        @update:options="loadTableData"
        @click:row="onRowClick"
      >
        <!-- Thumbnail Column -->
        <template #[`item.thumbnail`]="{ item }">
          <div class="thumbnail-wrapper">
            <thumbnail-img v-if="item.id" :media-item-id="item.id" class="table-thumbnail" cover />
            <v-icon v-else size="small" color="primary">mdi-image-outline</v-icon>
          </div>
        </template>

        <!-- Size Column -->
        <template #[`item.sizeBytes`]="{ item }">
          <span class="monospace-text">
            {{ item.sizeBytes !== null ? prettyBytes(item.sizeBytes) : '-' }}
          </span>
        </template>

        <!-- Weather: Temp Column -->
        <template #[`item.temperature`]="{ item }">
          <div v-if="item.temperature !== null" class="d-flex align-center">
            <v-icon icon="mdi-thermometer" size="small" color="orange" class="mr-1" />
            <span>{{ item.temperature.toFixed(1) }}°C</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Weather: Wind Speed Column -->
        <template #[`item.windSpeed`]="{ item }">
          <div v-if="item.windSpeed !== null" class="d-flex align-center">
            <v-icon icon="mdi-weather-windy" size="small" color="blue-lighten-2" class="mr-1" />
            <span>{{ item.windSpeed.toFixed(1) }} km/h</span>
          </div>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Camera Settings: ISO Column -->
        <template #[`item.iso`]="{ item }">
          <span v-if="item.iso !== null" class="monospace-text font-weight-medium">{{
            item.iso
          }}</span>
          <span v-else class="text-disabled">-</span>
        </template>

        <!-- Camera Settings: Shutter Speed Column -->
        <template #[`item.exposureTime`]="{ item }">
          <span class="monospace-text">{{ formatShutterSpeed(item.exposureTime) }}</span>
        </template>

        <!-- Camera Settings: Focal Length Column -->
        <template #[`item.focalLength`]="{ item }">
          <span class="monospace-text">{{ formatFocalLength(item.focalLength) }}</span>
        </template>

        <!-- GPS: Altitude Column -->
        <template #[`item.altitude`]="{ item }">
          <div
            v-if="item.altitude !== null"
            class="d-flex align-center"
            :title="formatCoords(item.latitude, item.longitude)"
          >
            <v-icon icon="mdi-image-filter-hdr" size="small" color="teal" class="mr-1" />
            <span>{{ Math.round(item.altitude) }}m</span>
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
  border-radius: 24px !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
  overflow: hidden;
}

.table-header-row {
  background-color: rgb(var(--v-theme-surface-container-high));
  padding: 16px 24px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.header-title-group {
  display: flex;
  align-items: center;
}

.header-title {
  font-size: 1.25rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.table-body {
  padding: 24px;
}

.explore-server-table {
  background: transparent !important;
}

.explore-server-table :deep(tbody tr) {
  cursor: pointer;
}

.thumbnail-wrapper {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1.5px solid rgba(var(--v-border-color), var(--v-border-opacity));
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
</style>
