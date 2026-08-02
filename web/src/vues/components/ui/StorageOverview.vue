<script setup lang="ts">
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const systemStore = useSystemStore()
const route = useRoute()

const diskStats = computed(() => systemStore.stats.disk)
const isStorageActive = computed(() => route.path.startsWith('/storage'))

const mediaUsedPct = computed(() => {
  const total = diskStats.value?.mediaDrive?.diskTotal || 0
  if (!total) return 0
  return Math.min(100, Math.max(0, (diskStats.value.mediaDrive.diskUsed / total) * 100))
})

const appDataUsedPct = computed(() => {
  const total = diskStats.value?.appDataDrive?.diskTotal || 0
  if (!total) return 0
  return Math.min(100, Math.max(0, (diskStats.value.appDataDrive.diskUsed / total) * 100))
})
</script>

<template>
  <router-link
    to="/storage"
    class="storage-card"
    v-ripple
    :class="{ 'storage-card--active': isStorageActive }"
  >
    <!-- Media / Main Storage Drive -->
    <div class="drive-section">
      <div class="drive-header">
        <div class="drive-title-wrap">
          <v-icon
            size="20"
            class="drive-icon"
            :color="isStorageActive ? 'primary' : 'on-surface-variant'"
          >
            mdi-cloud-outline
          </v-icon>
          <span class="drive-title">{{ diskStats.areSameDrive ? 'Storage' : 'Media' }}</span>
        </div>
        <span class="pct-badge"> {{ mediaUsedPct.toFixed(0) }}% </span>
      </div>

      <v-progress-linear
        :model-value="mediaUsedPct"
        color="primary"
        height="5"
        rounded
        class="storage-progress"
      />

      <div class="drive-footer">
        <span>
          {{ prettyBytes(diskStats.mediaDrive.diskUsed, 1) }} of
          {{ prettyBytes(diskStats.mediaDrive.diskTotal, 1) }}
        </span>
        <span>{{ prettyBytes(diskStats.mediaDrive.diskAvailable, 1) }} free</span>
      </div>
    </div>

    <!-- App Data Drive (when separate) -->
    <template v-if="!diskStats.areSameDrive">
      <v-divider class="my-3 opacity-20" />
      <div class="drive-section">
        <div class="drive-header">
          <div class="drive-title-wrap">
            <v-icon size="18" color="on-surface-variant" class="drive-icon">
              mdi-folder-outline
            </v-icon>
            <span class="drive-title text-caption font-weight-medium">App Data</span>
          </div>
          <span class="pct-badge"> {{ appDataUsedPct.toFixed(0) }}% </span>
        </div>

        <v-progress-linear
          :model-value="appDataUsedPct"
          color="primary"
          height="4"
          rounded
          class="storage-progress"
        />

        <div class="drive-footer">
          <span>
            {{ prettyBytes(diskStats.appDataDrive.diskUsed, 1) }} of
            {{ prettyBytes(diskStats.appDataDrive.diskTotal, 1) }}
          </span>
          <span>{{ prettyBytes(diskStats.appDataDrive.diskAvailable, 1) }} free</span>
        </div>
      </div>
    </template>
  </router-link>
</template>

<style scoped>
.storage-card {
  display: block;
  text-decoration: none;
  color: inherit;
  padding: 12px;
  margin-top: auto;
  margin-bottom: 8px;
  transition:
    background-color 0.2s ease,
    border-color 0.2s ease,
    transform 0.15s ease;
  user-select: none;
  border-radius: 20px;
}

.drive-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.drive-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.drive-title-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
}

.drive-icon {
  flex-shrink: 0;
  color: rgb(var(--v-theme-on-background));
  opacity: 0.6;
}

.drive-title {
  font-size: 0.9rem;
  font-weight: 600;
  line-height: 1.2;
  color: rgb(var(--v-theme-on-background));
  opacity: 0.8;
}

.pct-badge {
  font-size: 0.725rem;
  font-weight: 600;
  padding: 2px 7px;
  border-radius: 8px;
  background-color: rgba(var(--v-theme-on-surface), 0.08);
  color: rgba(var(--v-theme-on-surface), .6);
}

.pct-badge--warning {
  background-color: rgba(var(--v-theme-warning), 0.2);
  color: rgb(var(--v-theme-warning));
}

.pct-badge--error {
  background-color: rgba(var(--v-theme-error), 0.2);
  color: rgb(var(--v-theme-error));
}

.storage-progress {
  margin: 2px 0;
  opacity: 0.9;
}

.drive-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 0.725rem;
  color: rgb(var(--v-theme-on-surface-variant));
  font-weight: 500;
}
</style>
