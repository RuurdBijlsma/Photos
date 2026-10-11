<script setup lang="ts">
import type { JobInfo } from '@/scripts/types/api/admin.ts'
import MdiAlertCircle from '~icons/mdi/alert-circle'
import MdiCached from '~icons/mdi/cached'
import MdiClose from '~icons/mdi/close'
import MdiDownloadOutline from '~icons/mdi/download-outline'
import MdiInformation from '~icons/mdi/information'

defineProps<{
  modelValue: boolean
  job: JobInfo | null
  isActionLoading: boolean
  isDownloading: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'retry', jobId: number): void
  (e: 'download', path: string, jobId: number): void
}>()

function formatJobType(jobType: string) {
  switch (jobType) {
    case 'ingest_metadata':
      return 'Metadata'
    case 'ingest_thumbnails':
      return 'Thumbnails'
    case 'ingest_analysis':
      return 'Analysis'
    default:
      return jobType
  }
}

function formatDate(dateStr: string | null) {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString()
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    max-width="700px"
  >
    <v-card rounded="xl" color="surface-container-highest" class="dialog-card">
      <v-card-title class="dialog-header">
        <div class="dialog-title">
          <v-icon
            :icon="job?.lastError ? MdiAlertCircle : MdiInformation"
            :color="job?.lastError ? 'error' : 'primary'"
            class="dialog-title-icon"
          />
          Job #{{ job?.id }} ({{ job?.jobType ? formatJobType(job.jobType) : '' }})
        </div>
        <v-btn
          :icon="MdiClose"
          variant="text"
          density="comfortable"
          @click="emit('update:modelValue', false)"
        />
      </v-card-title>

      <v-card-text class="dialog-body">
        <div class="dialog-error-section">
          <div class="dialog-section-label">File Path</div>
          <code class="path-display">{{ job?.relativePath || '-' }}</code>
        </div>

        <div v-if="job?.lastError" class="dialog-error-section">
          <div class="error-label">Error Log</div>
          <pre class="console-box error-box">{{ job.lastError }}</pre>
        </div>

        <div>
          <div class="dialog-section-label">Parameters (Payload)</div>
          <pre class="console-box">{{ JSON.stringify(job?.payload, null, 2) }}</pre>
        </div>

        <v-row class="dialog-metadata" density="comfortable">
          <v-col cols="12" sm="4">
            <strong>Created:</strong><br />
            {{ formatDate(job?.createdAt || '') }}
          </v-col>
          <v-col cols="6" sm="4">
            <strong>Started:</strong><br />
            {{ formatDate(job?.startedAt || '') }}
          </v-col>
          <v-col cols="6" sm="4">
            <strong>Finished:</strong><br />
            {{ formatDate(job?.finishedAt || '') }}
          </v-col>
        </v-row>
      </v-card-text>

      <v-card-actions class="dialog-actions">
        <v-btn
          v-if="
            job && (job.status === 'failed' || job.status === 'done' || job.status === 'cancelled')
          "
          variant="tonal"
          color="primary"
          :prepend-icon="MdiCached"
          rounded="xl"
          :loading="isActionLoading"
          @click="emit('retry', job.id)"
          class="dialog-action-btn"
        >
          Retry Job
        </v-btn>
        <v-btn
          v-if="job?.relativePath"
          variant="tonal"
          color="secondary"
          :prepend-icon="MdiDownloadOutline"
          rounded="xl"
          :loading="isDownloading"
          @click="emit('download', job.relativePath, job.id)"
          class="dialog-action-btn"
        >
          Download File
        </v-btn>
        <v-spacer />
        <v-btn
          color="secondary"
          variant="text"
          rounded="xl"
          @click="emit('update:modelValue', false)"
        >
          Close
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.dialog-card {
  border: 1px solid rgba(var(--v-border-color), 0.12) !important;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
}

.is-mobile .dialog-header {
  padding: 14px 16px;
}

.dialog-title {
  display: flex;
  align-items: center;
  font-weight: 700;
  font-size: 1.25rem;
}

.is-mobile .dialog-title {
  font-size: 1.05rem;
}

.dialog-title-icon {
  margin-right: 8px;
}

.dialog-body {
  padding: 16px 24px;
}

.is-mobile .dialog-body {
  padding: 14px 16px;
}

.dialog-section-label {
  font-size: 0.875rem;
  font-weight: 700;
  color: rgba(var(--v-theme-on-surface), 0.6);
  margin-bottom: 4px;
}

.path-display {
  background-color: rgb(var(--v-theme-surface-container-lowest));
  font-family: monospace;
  font-size: 0.85rem;
  display: inline-block;
  word-break: break-all;
  padding: 4px 8px;
  border-radius: 4px;
}

.dialog-error-section {
  margin-bottom: 16px;
}

.error-label {
  color: rgb(var(--v-theme-error));
  font-size: 0.875rem;
  font-weight: 700;
  margin-bottom: 4px;
}

.console-box {
  background: rgb(var(--v-theme-surface-container-lowest));
  border: 1px solid rgba(var(--v-border-color), 0.1);
  border-radius: 12px;
  overflow-x: auto;
  font-family: monospace;
  font-size: 0.8rem;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 220px;
  overflow-y: auto;
  padding: 12px;
}

.error-box {
  color: rgb(var(--v-theme-error));
  border-color: rgba(var(--v-theme-error), 0.15);
  background-color: rgba(var(--v-theme-error), 0.02);
}

.dialog-metadata {
  margin-top: 16px;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.dialog-actions {
  padding: 16px 24px 24px 24px;
  display: flex;
  align-items: center;
  border-top: 1px solid rgba(var(--v-border-color), 0.12);
}

.is-mobile .dialog-actions {
  padding: 12px 16px 16px 16px;
  flex-wrap: wrap;
  gap: 8px;
}

.dialog-action-btn {
  margin-right: 8px;
}
</style>
