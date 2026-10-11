<script setup lang="ts">
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import RunningJobPill from '@/vues/components/activity/RunningJobPill.vue'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'
import MdiCheckCircleOutline from '~icons/mdi/check-circle-outline'

const ingestStore = useIngestJobsStore()
const { isMobile } = useResponsive()
</script>

<template>
  <v-card class="action-card" flat>
    <div class="card-body">
      <h2 class="feed-title">
        <span
          >Currently importing ({{ ingestStore.runningJobs.length
          }}{{ ingestStore.runningJobs.length >= 100 ? '+' : '' }})</span
        >
      </h2>

      <div class="running-list-container">
        <div v-if="ingestStore.runningJobs.length > 0" class="running-jobs-wrap">
          <RunningJobPill
            bg-color="surface-container-high"
            v-for="job in ingestStore.runningJobs.slice(0, 15)"
            :key="job.id"
            :job-type="job.jobType"
            :relative-path="job.relativePath"
            :compact="isMobile"
          />
          <div v-if="ingestStore.runningJobs.length > 15" class="background-tasks-indicator">
            + {{ ingestStore.runningJobs.length - 15 }} more active tasks in background
          </div>
        </div>
        <div v-else class="running-empty-state">
          <v-icon size="large" class="empty-state-icon" :icon="MdiCheckCircleOutline" />
          <div class="empty-state-text">No active background ingestion tasks running.</div>
        </div>
      </div>
    </div>
  </v-card>
</template>

<style scoped>
.action-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  margin-bottom: 24px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.is-mobile .action-card {
  border-radius: 22px !important;
  margin-bottom: 16px;
}

.card-body {
  padding: 20px;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
}

.is-mobile .card-body {
  padding: 16px 14px;
}

.feed-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.is-mobile .feed-title {
  font-size: 1.1rem;
}

.running-list-container {
  min-height: 140px;
  width: 100%;
}

.running-jobs-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 240px;
  overflow-y: auto;
  padding-right: 4px;
  width: 100%;
}

.background-tasks-indicator {
  text-align: center;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
  padding: 8px 0;
}

.running-empty-state {
  text-align: center;
  padding: 32px 0;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.empty-state-icon {
  opacity: 0.4;
  margin-bottom: 8px;
}

.empty-state-text {
  font-size: 0.875rem;
  font-weight: 500;
}
</style>
