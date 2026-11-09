<script setup lang="ts">
import { computed } from 'vue'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import IngestDashboard from '@/vues/components/activity/IngestDashboard.vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useEventListener } from '@vueuse/core'
import MdiBackupRestore from '~icons/mdi/backup-restore'

const uploadStore = useUploadStore()
const ingestStore = useIngestJobsStore()

const queuedCount = computed(() => {
  const overview = ingestStore.overview
  if (!overview) return 0
  return overview.metadata.queued + overview.thumbnails.queued + overview.analysis.queued
})

async function handleCancelPending() {
  try {
    await ingestStore.cancelPendingJobs()
  } catch {
    // Managed in store
  }
}

function handleBeforeUnload(e: BeforeUnloadEvent) {
  if (uploadStore.isUploading) {
    e.preventDefault()
    e.returnValue = ''
  }
}

useEventListener('beforeunload', handleBeforeUnload)
</script>

<template>
  <main-layout-container class="activity-scroll-view">
    <div class="activity-content">
      <header class="activity-header mb-6">
        <div>
          <h1 class="activity-title">Library import</h1>
          <p class="activity-subtitle">
            Monitor your server's metadata extraction, image processing, and analysis pipelines.
          </p>
        </div>
        <v-tooltip v-if="queuedCount > 0" text="Cancel queued jobs and rescan" location="bottom">
          <template #activator="{ props }">
            <v-btn
              v-bind="props"
              :prepend-icon="MdiBackupRestore"
              aria-label="Cancel queued jobs and rescan"
              color="error"
              variant="plain"
              rounded="xl"
              :loading="ingestStore.isCancelling"
              @click="handleCancelPending"
            >
              Cancel queued jobs
            </v-btn>
          </template>
        </v-tooltip>
      </header>

      <ingest-dashboard />
    </div>
  </main-layout-container>
</template>

<style scoped>
.activity-scroll-view {
  overflow-y: auto;
  overflow-x: hidden;
  width: 100%;
  max-width: 100%;
}

.activity-content {
  max-width: 1400px;
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
  margin: 0 auto;
  padding: 32px 24px;
}

.is-mobile .activity-content {
  padding: 16px 12px 96px 12px;
}

.activity-title {
  font-size: 2.125rem;
  font-weight: 700;
  margin-bottom: 6px;
  color: rgb(var(--v-theme-on-surface));
}

.activity-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.is-mobile .activity-title {
  font-size: 1.5rem;
  margin-bottom: 4px;
}

.activity-subtitle {
  font-size: 0.95rem;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 0;
}

.is-mobile .activity-subtitle {
  font-size: 0.825rem;
}

.is-mobile .activity-header {
  flex-direction: column;
  align-items: flex-start;
  margin-bottom: 16px !important;
}
</style>
