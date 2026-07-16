<script setup lang="ts">
import { ref } from 'vue'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import type { ExploreMediaItem } from '@/scripts/types/api/explore.ts'

const exploreStore = useExploreStore()

// Local modal for media detail preview
const previewDialog = ref(false)
const previewItem = ref<ExploreMediaItem | null>(null)

// Headers mapped directly to our secure whitelisted database columns
const headers = [
  { title: 'Preview', key: 'thumbnail', sortable: false, width: '80px' },
  { title: 'Filename', key: 'filename', sortable: true },
  { title: 'Date Taken', key: 'takenAtLocal', sortable: true },
  { title: 'Size', key: 'sizeBytes', sortable: true },
  { title: 'Temp', key: 'temperature', sortable: true },
  { title: 'Wind', key: 'windSpeed', sortable: true },
  { title: 'ISO', key: 'iso', sortable: true },
  { title: 'Shutter', key: 'exposureTime', sortable: true },
  { title: 'Aperture', key: 'aperture', sortable: true },
  { title: 'Focal', key: 'focalLength', sortable: true },
  { title: 'Altitude', key: 'altitude', sortable: true },
]

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

function openPreview(item: ExploreMediaItem) {
  previewItem.value = item
  previewDialog.value = true
}

function closePreview() {
  previewDialog.value = false
  previewItem.value = null
}

function formatShutterSpeed(seconds: number | null): string {
  if (seconds === null || seconds === undefined) return '-'
  if (seconds >= 1) {
    return `${seconds.toFixed(1)}s`
  }
  const fraction = Math.round(1 / seconds)
  return `1/${fraction}s`
}

function formatAperture(fNumber: number | null): string {
  if (fNumber === null || fNumber === undefined) return '-'
  return `f/${fNumber.toFixed(1)}`
}

function formatFocalLength(mm: number | null): string {
  if (mm === null || mm === undefined) return '-'
  return `${Math.round(mm)}mm`
}

function formatCoords(lat: number | null, lon: number | null): string {
  if (lat === null || lon === null) return '-'
  return `${lat.toFixed(4)}, ${lon.toFixed(4)}`
}

function formatDateTime(dateStr: string) {
  return new Date(dateStr).toLocaleString()
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
      >
        <!-- Thumbnail Column -->
        <template #[`item.thumbnail`]="{ item }">
          <div class="thumbnail-wrapper" @click="openPreview(item)">
            <thumbnail-img v-if="item.id" :media-item-id="item.id" class="table-thumbnail" cover />
            <v-icon v-else size="small" color="primary">mdi-image-outline</v-icon>
          </div>
        </template>

        <!-- Filename Column -->
        <template #[`item.filename`]="{ item }">
          <span
            class="monospace-text text-truncate d-inline-block filename-col"
            :title="item.filename"
          >
            {{ item.filename }}
          </span>
        </template>

        <!-- Date Taken Column -->
        <template #[`item.takenAtLocal`]="{ item }">
          <span class="datetime-text">
            {{ formatDateTime(item.takenAtLocal) }}
          </span>
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

        <!-- Camera Settings: Aperture Column -->
        <template #[`item.aperture`]="{ item }">
          <span class="monospace-text">{{ formatAperture(item.aperture) }}</span>
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

    <!-- Modal Dialog: Self-Contained Photo/Video Stats Highlight and Preview -->
    <v-dialog v-model="previewDialog" max-width="650px">
      <v-card rounded="xl" color="surface-container-highest" class="border">
        <v-card-title class="dialog-header d-flex align-center justify-space-between py-4 px-6">
          <div class="d-flex align-center font-weight-bold truncate-title">
            <v-icon icon="mdi-eye-outline" color="primary" class="mr-2" />
            {{ previewItem?.filename }}
          </div>
          <v-btn icon="mdi-close" variant="text" density="comfortable" @click="closePreview" />
        </v-card-title>

        <v-card-text class="py-4 px-6 text-center">
          <div class="preview-img-container mb-4">
            <thumbnail-img
              v-if="previewItem?.id"
              :media-item-id="previewItem.id"
              class="modal-preview-img"
              contain
            />
          </div>

          <!-- Highlight Stats Cards Grid -->
          <v-row density="comfortable" class="stats-row">
            <v-col cols="6" sm="4" class="stat-box">
              <v-icon icon="mdi-calendar" color="secondary" size="small" />
              <div class="stat-val">
                {{ previewItem ? formatDateTime(previewItem.takenAtLocal) : '' }}
              </div>
              <div class="stat-lbl">Date Taken</div>
            </v-col>
            <v-col cols="6" sm="4" class="stat-box" v-if="previewItem?.temperature !== null">
              <v-icon icon="mdi-thermometer" color="orange" size="small" />
              <div class="stat-val">{{ previewItem?.temperature?.toFixed(1) }}°C</div>
              <div class="stat-lbl">Temperature</div>
            </v-col>
            <v-col cols="6" sm="4" class="stat-box" v-if="previewItem?.altitude !== null">
              <v-icon icon="mdi-image-filter-hdr" color="teal" size="small" />
              <div class="stat-val">
                {{ previewItem?.altitude ? Math.round(previewItem.altitude) : 0 }}m
              </div>
              <div class="stat-lbl">Altitude</div>
            </v-col>
            <v-col cols="6" sm="4" class="stat-box" v-if="previewItem?.iso">
              <v-icon icon="mdi-camera" color="blue" size="small" />
              <div class="stat-val">ISO {{ previewItem?.iso }}</div>
              <div class="stat-lbl">Sensitivity</div>
            </v-col>
            <v-col cols="6" sm="4" class="stat-box" v-if="previewItem?.exposureTime">
              <v-icon icon="mdi-shutter-speed" color="purple" size="small" />
              <div class="stat-val">{{ formatShutterSpeed(previewItem?.exposureTime) }}</div>
              <div class="stat-lbl">Shutter Speed</div>
            </v-col>
            <v-col cols="6" sm="4" class="stat-box" v-if="previewItem?.sizeBytes">
              <v-icon icon="mdi-weight" color="green" size="small" />
              <div class="stat-val">{{ prettyBytes(previewItem.sizeBytes) }}</div>
              <div class="stat-lbl">File Size</div>
            </v-col>
          </v-row>
        </v-card-text>

        <v-card-actions class="px-6 pb-6 d-flex justify-end">
          <v-btn color="primary" variant="text" rounded @click="closePreview">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
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

.thumbnail-wrapper {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  overflow: hidden;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1.5px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition: transform 0.15s ease;
}

.thumbnail-wrapper:hover {
  transform: scale(1.05);
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

.filename-col {
  max-width: 180px;
}

.datetime-text {
  font-size: 0.85rem;
}

.truncate-title {
  max-width: 80%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Modal styling */
.preview-img-container {
  max-height: 400px;
  width: 100%;
  border-radius: 16px;
  overflow: hidden;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  background-color: #0c0c0c;
  display: flex;
  justify-content: center;
  align-items: center;
}

.modal-preview-img {
  max-height: 400px;
  max-width: 100%;
  object-fit: contain;
}

.stats-row {
  background-color: rgb(var(--v-theme-surface-container-low));
  border-radius: 16px;
  padding: 8px;
  margin: 0 !important;
}

.stat-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 12px 6px !important;
}

.stat-val {
  font-weight: 700;
  font-size: 0.9rem;
  margin-top: 4px;
  color: rgb(var(--v-theme-on-surface));
}

.stat-lbl {
  font-size: 0.75rem;
  color: rgb(var(--v-theme-on-surface-variant));
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-top: 2px;
}

.dialog-header {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.15);
}
</style>
