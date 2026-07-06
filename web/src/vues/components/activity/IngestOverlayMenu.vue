<script setup lang="ts">
import { onMounted, onUnmounted, computed } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import IngestPipelineRow from '@/vues/components/activity/IngestPipelineRow.vue'
import RunningJobPill from '@/vues/components/activity/RunningJobPill.vue'

const emit = defineEmits<{
  (e: 'close-menu'): void
}>()

const ingestStore = useIngestJobsStore()

onMounted(() => {
  ingestStore.startPolling(false)
})

onUnmounted(() => {
  ingestStore.stopPolling(false)
})

const categories = [
  { id: 'metadata', label: 'File import', icon: 'mdi-file-image-outline' },
  { id: 'thumbnails', label: 'Generate thumbnails', icon: 'mdi-image-outline' },
  { id: 'analysis', label: 'Index for search', icon: 'mdi-search-web' },
] as const

const activeCategories = computed(() => {
  if (!ingestStore.overview) return new Set<string>()
  const activeCats = new Set<string>()
  for (const cat of categories) {
    const counts = ingestStore.overview[cat.id]
    if (counts && counts.running > 0) {
      activeCats.add(cat.id)
    }
  }
  return activeCats
})

const filteredCategoryProgress = computed(() => {
  if (!ingestStore.overview) return []

  const progress = categories.map((cat) => {
    const counts = ingestStore.overview![cat.id]
    const total = counts?.total || 0
    const done = counts?.done || 0
    const running = counts?.running || 0
    const queued = counts?.queued || 0
    const toGo = queued + running
    const percentage = total > 0 ? Math.floor((done / total) * 100) : 0

    return {
      ...cat,
      total,
      toGo,
      percentage,
      counts,
    }
  })

  if (activeCategories.value.size > 0) {
    return progress.filter((cat) => activeCategories.value.has(cat.id))
  }
  return progress
})
</script>

<template>
  <div class="overlay-container">
    <div class="ingest-card">
      <div class="ingest-header">
        <span class="font-weight-bold">Importing photos and videos...</span>
        <v-btn
          icon="mdi-refresh"
          variant="text"
          density="comfortable"
          color="primary"
          @click="ingestStore.pollTick"
          :loading="ingestStore.isOverviewLoading || ingestStore.isRunningLoading"
        />
      </div>

      <div class="progress-section">
        <IngestPipelineRow
          v-for="cat in filteredCategoryProgress"
          :key="cat.id"
          v-bind="cat"
          compact
        />
      </div>

      <div class="running-jobs-section">
        <div class="section-title">
          <v-icon icon="mdi-swap-vertical" color="primary" />
          <span>Currently importing</span>
        </div>

        <div v-if="ingestStore.runningJobs.length > 0">
          <RunningJobPill
            v-for="job in ingestStore.runningJobs.slice(0, 4)"
            :key="job.id"
            :job-type="job.jobType"
            :relative-path="job.relativePath"
            compact
          />
          <div v-if="ingestStore.runningJobs.length > 4" class="more-tasks">
            + {{ ingestStore.runningJobs.length - 4 }} more active tasks
          </div>
        </div>
      </div>

      <div class="mt-4 pt-2">
        <v-btn
          to="/activity"
          color="primary"
          block
          variant="tonal"
          rounded="xl"
          prepend-icon="mdi-arrow-right"
          @click="emit('close-menu')"
        >
          View Full Activity
        </v-btn>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay-container {
  width: 420px;
  max-width: 100vw;
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
  border-radius: 24px !important;
  padding: 16px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35) !important;
}

.ingest-card {
  display: flex;
  flex-direction: column;
}

.ingest-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.progress-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.running-jobs-section {
  margin-top: 16px;
}

.section-title {
  font-size: 1.05rem;
  font-weight: 600;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.more-tasks {
  text-align: center;
  font-weight: 400;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 14px;
}
</style>
