<script setup lang="ts">
import { prettyBytes } from '@/scripts/utils.ts'
import { computed } from 'vue'
import type { DiskStats } from '@/scripts/types/api/system.ts'

const props = defineProps<{
  mediaFolderSizeBytes: number
  appDataFolderSizeBytes: number
  diskStats: DiskStats
}>()

const mediaUsedPercentage = computed(() =>
  percentage(props.diskStats.mediaDrive.diskUsed, props.diskStats.mediaDrive.diskTotal),
)
const appDataUsedPercentage = computed(() =>
  percentage(props.diskStats.appDataDrive.diskUsed, props.diskStats.appDataDrive.diskTotal),
)

// Computed breakdown for media drive usage
const mediaDriveUsage = computed(() => {
  const drive = props.diskStats.mediaDrive
  const total = drive.diskTotal
  const used = drive.diskUsed

  // Guard against out-of-sync backend measurements and division by zero
  const mediaBytes = Math.min(used, props.mediaFolderSizeBytes)
  const appDataBytes = props.diskStats.areSameDrive
    ? Math.min(used - mediaBytes, props.appDataFolderSizeBytes)
    : 0

  const otherBytes = Math.max(0, used - mediaBytes - appDataBytes)

  const mediaPercent = total > 0 ? (mediaBytes / total) * 100 : 0
  const appDataPercent = total > 0 ? (appDataBytes / total) * 100 : 0
  const otherPercent = total > 0 ? (otherBytes / total) * 100 : 0

  return {
    mediaBytes,
    appDataBytes,
    otherBytes,
    mediaPercent,
    appDataPercent,
    otherPercent,
  }
})

// Computed breakdown for app_data drive usage (only relevant when drives are separate)
const appDataDriveUsage = computed(() => {
  const drive = props.diskStats.appDataDrive
  const total = drive.diskTotal
  const used = drive.diskUsed

  const appDataBytes = Math.min(used, props.appDataFolderSizeBytes)
  const otherBytes = Math.max(0, used - appDataBytes)

  const appDataPercent = total > 0 ? (appDataBytes / total) * 100 : 0
  const otherPercent = total > 0 ? (otherBytes / total) * 100 : 0

  return {
    appDataBytes,
    otherBytes,
    appDataPercent,
    otherPercent,
  }
})

function percentage(used: number, total: number) {
  if (total <= 0) return 0
  return Math.min(100, Math.max(0, (used / total) * 100))
}
</script>

<template>
  <section class="usage-section">
    <!-- Media / Unified Storage Card -->
    <div class="usage-card">
      <div class="usage-card-header">
        <div>
          <h2>{{ diskStats.areSameDrive ? 'Storage usage' : 'Media drive' }}</h2>
          <div class="usage-used-available">
            <span>
              {{ prettyBytes(diskStats.mediaDrive.diskUsed, 1) }} of
              {{ prettyBytes(diskStats.mediaDrive.diskTotal, 1) }} used
            </span>
            <span>•</span>
            <span> {{ prettyBytes(diskStats.mediaDrive.diskAvailable, 1) }} available </span>
          </div>
        </div>
        <strong class="usage-percentage">{{ Math.round(mediaUsedPercentage) }}%</strong>
      </div>

      <!-- Multi-segment Progress Bar -->
      <div class="storage-progress-bar">
        <div
          v-if="mediaDriveUsage.mediaPercent > 0"
          class="progress-segment media-segment"
          :style="{ width: mediaDriveUsage.mediaPercent + '%' }"
          v-tooltip:top="'Media folder'"
        />
        <div
          v-if="mediaDriveUsage.appDataPercent > 0"
          class="progress-segment thumbnail-segment"
          :style="{ width: mediaDriveUsage.appDataPercent + '%' }"
          v-tooltip:top="'App data folder'"
        />
        <div
          v-if="mediaDriveUsage.otherPercent > 0"
          class="progress-segment other-segment"
          :style="{ width: mediaDriveUsage.otherPercent + '%' }"
          v-tooltip:top="'Other system files & data'"
        />
      </div>

      <!-- Legend -->
      <div class="legend-container">
        <div class="legend-item" v-if="mediaDriveUsage.mediaBytes > 0">
          <span class="legend-dot media-dot"></span>
          <span class="legend-label">Media:</span>
          <strong class="legend-value">{{ prettyBytes(mediaDriveUsage.mediaBytes, 1) }}</strong>
        </div>
        <div class="legend-item" v-if="mediaDriveUsage.appDataBytes > 0">
          <span class="legend-dot thumbnail-dot"></span>
          <span class="legend-label">App data:</span>
          <strong class="legend-value">{{ prettyBytes(mediaDriveUsage.appDataBytes, 1) }}</strong>
        </div>
        <div class="legend-item" v-if="mediaDriveUsage.otherBytes > 0">
          <span class="legend-dot other-dot"></span>
          <span class="legend-label">Other files:</span>
          <strong class="legend-value">{{ prettyBytes(mediaDriveUsage.otherBytes, 1) }}</strong>
        </div>
      </div>
    </div>

    <!-- Dedicated Thumbnail Storage Card -->
    <div class="usage-card" v-if="!diskStats.areSameDrive">
      <div class="usage-card-header">
        <div>
          <h2>App data drive</h2>
          <div class="usage-used-available">
            <span>
              {{ prettyBytes(diskStats.appDataDrive.diskUsed, 1) }} of
              {{ prettyBytes(diskStats.appDataDrive.diskTotal, 1) }} used
            </span>
            <span>•</span>
            <span> {{ prettyBytes(diskStats.appDataDrive.diskAvailable, 1) }} available </span>
          </div>
        </div>
        <strong class="usage-percentage">{{ Math.round(appDataUsedPercentage) }}%</strong>
      </div>

      <!-- Multi-segment Progress Bar -->
      <div class="storage-progress-bar">
        <div
          v-if="appDataDriveUsage.appDataPercent > 0"
          class="progress-segment thumbnail-segment"
          :style="{ width: appDataDriveUsage.appDataPercent + '%' }"
          v-tooltip:top="'App data folder'"
        />
        <div
          v-if="appDataDriveUsage.otherPercent > 0"
          class="progress-segment other-segment"
          :style="{ width: appDataDriveUsage.otherPercent + '%' }"
          v-tooltip:top="'Other system files & data'"
        />
      </div>

      <!-- Legend -->
      <div class="legend-container">
        <div class="legend-item" v-if="appDataDriveUsage.appDataBytes > 0">
          <span class="legend-dot thumbnail-dot"></span>
          <span class="legend-label">App data:</span>
          <strong class="legend-value">{{ prettyBytes(appDataDriveUsage.appDataBytes, 1) }}</strong>
        </div>
        <div class="legend-item" v-if="appDataDriveUsage.otherBytes > 0">
          <span class="legend-dot other-dot"></span>
          <span class="legend-label">Other files:</span>
          <strong class="legend-value">{{ prettyBytes(appDataDriveUsage.otherBytes, 1) }}</strong>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.usage-section {
  --media-color: #21b655;
  --thumb-color: #dd6120;
  --other-color: rgba(var(--v-theme-on-surface-variant), 0.5);
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
  margin-bottom: 34px;
}

.usage-card {
  padding: 20px;
  border-radius: 28px;
  background: rgb(var(--v-theme-surface-container-low));
}

.usage-card-header {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  align-items: flex-start;
  margin-bottom: 16px;
}

.usage-used-available {
  display: flex;
  align-items: center;
  color: rgb(var(--v-theme-on-surface-variant));
  gap: 10px;
}

.usage-percentage {
  font-size: 18px;
  color: rgb(var(--v-theme-on-surface-variant));
  font-weight: 500;
  margin: 10px;
}

/* Custom Progress Bar Styling */
.storage-progress-bar {
  display: flex;
  height: 14px;
  width: 100%;
  background-color: rgba(var(--v-theme-on-surface), 0.08);
  border-radius: 999px;
  overflow: hidden;
  margin-bottom: 12px;
}

.progress-segment {
  height: 100%;
  transition: width 0.3s ease;
}

.media-segment {
  background-color: var(--media-color);
}

.thumbnail-segment {
  background-color: var(--thumb-color);
}

.other-segment {
  background-color: var(--other-color);
}

/* Legend Styling */
.legend-container {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  margin-top: 8px;
  font-size: 0.85rem;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.media-dot {
  background-color: var(--media-color);
}

.thumbnail-dot {
  background-color: var(--thumb-color);
}

.other-dot {
  background-color: var(--other-color);
}

.legend-label {
  color: rgb(var(--v-theme-on-surface-variant));
}

.legend-value {
  font-weight: 600;
}
</style>
