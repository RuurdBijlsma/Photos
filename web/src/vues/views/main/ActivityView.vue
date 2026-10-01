<script setup lang="ts">
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import IngestDashboard from '@/vues/components/activity/IngestDashboard.vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { useEventListener } from '@vueuse/core'

const uploadStore = useUploadStore()

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
        <h1 class="activity-title">Library import</h1>
        <p class="activity-subtitle">
          Monitor your server's metadata extraction, image processing, and analysis pipelines.
        </p>
      </header>

      <ingest-dashboard />
    </div>
  </main-layout-container>
</template>

<style scoped>
.activity-scroll-view {
  overflow-y: auto;
}

.activity-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 32px 24px;
}

.is-mobile .activity-content {
  padding: 16px 14px 96px 14px;
}

.activity-title {
  font-size: 2.125rem;
  font-weight: 700;
  margin-bottom: 6px;
  color: rgb(var(--v-theme-on-surface));
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
  margin-bottom: 16px !important;
}
</style>
