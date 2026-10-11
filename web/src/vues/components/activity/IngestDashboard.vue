<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import IngestPipelineOverview from '@/vues/components/activity/cards/IngestPipelineOverview.vue'
import IngestUploadCard from '@/vues/components/activity/cards/IngestUploadCard.vue'
import IngestScanCard from '@/vues/components/activity/cards/IngestScanCard.vue'
import IngestActiveUploadsCard from '@/vues/components/activity/cards/IngestActiveUploadsCard.vue'
import IngestRunningFeed from '@/vues/components/activity/cards/IngestRunningFeed.vue'
import IngestQueueTable from '@/vues/components/activity/cards/IngestQueueTable.vue'
import MdiAlertCircleOutline from '~icons/mdi/alert-circle-outline'

const ingestStore = useIngestJobsStore()
const systemStore = useSystemStore()

const mediaFolderUnavailable = computed(
  () =>
    ingestStore.overview?.mediaFolder?.available === false ||
    systemStore.stats.mediaFolderAvailable === false,
)

const mediaFolderError = computed(
  () =>
    ingestStore.overview?.mediaFolder?.reason ||
    systemStore.stats.mediaFolderError ||
    'The media folder looks unmounted or empty. Ingest is paused until it is available again.',
)

onMounted(() => {
  ingestStore.startPolling(true)
  ingestStore.fetchUserJobs(true)
})

onUnmounted(() => {
  ingestStore.stopPolling(true)
})
</script>

<template>
  <div class="dashboard-root">
    <v-alert
      v-if="mediaFolderUnavailable"
      type="error"
      variant="tonal"
      class="mb-4"
      :icon="MdiAlertCircleOutline"
    >
      {{ mediaFolderError }}
    </v-alert>

    <!-- Top Pipeline Section -->
    <IngestPipelineOverview />

    <!-- Bottom Layout Grid -->
    <div class="dashboard-grid">
      <!-- Left Column: Dropzone, Scan, Upload Progress -->
      <div class="grid-column">
        <IngestUploadCard />
        <IngestActiveUploadsCard />
        <IngestScanCard />
      </div>

      <!-- Right Column: Currently Ingesting Feed & Ingest Jobs Table -->
      <div class="grid-column">
        <IngestRunningFeed />
        <IngestQueueTable />
      </div>
    </div>
  </div>
</template>

<style scoped>
.dashboard-root {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 28px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.is-mobile .dashboard-grid {
  gap: 16px;
}

@media (min-width: 1024px) {
  .dashboard-grid {
    grid-template-columns: minmax(0, 5fr) minmax(0, 6fr);
  }
}

.grid-column {
  display: flex;
  flex-direction: column;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
}
</style>
