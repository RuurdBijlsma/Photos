<script setup lang="ts">
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import { computed } from 'vue'

const systemStore = useSystemStore()
const diskStats = computed(() => systemStore.stats.disk)
const usedPercentage = computed(
  () => (diskStats.value.mediaDrive.diskUsed / diskStats.value.mediaDrive.diskTotal) * 100,
)
const thumbsUsedPercentage = computed(
  () => (diskStats.value.appDataDrive.diskUsed / diskStats.value.appDataDrive.diskTotal) * 100,
)
</script>

<template>
  <div class="storage-container">
    <div class="storage-info">
      <p v-if="!diskStats.areSameDrive" class="drive-descriptor">Media drive</p>
      <v-progress-linear
        class="progress-linear"
        :model-value="usedPercentage"
        color="primary"
        rounded-bar
      />
      <p class="usage-text">
        {{ prettyBytes(diskStats.mediaDrive.diskUsed, 1) }} of
        {{ prettyBytes(diskStats.mediaDrive.diskTotal, 1) }} used
      </p>
    </div>
    <div class="storage-info" v-if="!diskStats.areSameDrive">
      <p class="drive-descriptor">App data drive</p>
      <v-progress-linear
        class="progress-linear"
        :model-value="thumbsUsedPercentage"
        color="primary"
        rounded-bar
      />
      <p class="usage-text">
        {{ prettyBytes(diskStats.appDataDrive.diskUsed, 1) }} of
        {{ prettyBytes(diskStats.appDataDrive.diskTotal, 1) }} used
      </p>
    </div>
  </div>
</template>

<style scoped>
.storage-info {
  padding: 15px;
  padding-bottom: 10px;
  padding-top: 0;
}

.progress-linear {
  opacity: 0.7;
}

.drive-descriptor {
  font-size: 12px;
  font-weight: 500;
}

.usage-text {
  margin-top: 6px;
  font-size: 12px;
  color: rgba(var(--v-theme-on-background), 0.8);
  font-weight: 400;
}
</style>
