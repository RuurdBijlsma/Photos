<script setup lang="ts">
import { computed } from 'vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import MdiAlertCircleOutline from '~icons/mdi/alert-circle-outline'
import MdiCheckCircleOutline from '~icons/mdi/check-circle-outline'
import MdiClockOutline from '~icons/mdi/clock-outline'
import MdiClose from '~icons/mdi/close'
import MdiCloudUploadOutline from '~icons/mdi/cloud-upload-outline'
import MdiPauseCircleOutline from '~icons/mdi/pause-circle-outline'
import MdiStop from '~icons/mdi/stop'
import MdiStopCircleOutline from '~icons/mdi/stop-circle-outline'

const uploadStore = useUploadStore()

const statusPriority: Record<string, number> = {
  uploading: 0,
  failed: 1,
  pending: 2,
  stopped: 3,
  success: 4,
}

const sortedUploads = computed(() => {
  return [...uploadStore.uploads].sort(
    (a, b) => (statusPriority[a.status] ?? 99) - (statusPriority[b.status] ?? 99),
  )
})

function getStatusColor(status: string) {
  switch (status) {
    case 'uploading':
    case 'running':
      return 'primary'
    case 'success':
    case 'done':
      return 'success'
    case 'failed':
      return 'error'
    case 'stopped':
    case 'cancelled':
      return 'warning'
    default:
      return 'grey'
  }
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'uploading':
      return MdiCloudUploadOutline
    case 'success':
    case 'done':
      return MdiCheckCircleOutline
    case 'failed':
      return MdiAlertCircleOutline
    case 'stopped':
    case 'cancelled':
      return MdiPauseCircleOutline
    default:
      return MdiClockOutline
  }
}
</script>

<template>
  <v-card v-if="uploadStore.uploads.length > 0" class="action-card" flat>
    <div class="card-body">
      <div class="active-uploads-header">
        <h2 class="card-title">Active Uploads</h2>
        <v-btn
          variant="tonal"
          color="error"
          size="small"
          rounded
          :prepend-icon="MdiStopCircleOutline"
          @click="uploadStore.abortAll"
        >
          Cancel All
        </v-btn>
      </div>

      <v-virtual-scroll
        :items="sortedUploads"
        height="350"
        item-height="62"
        class="active-uploads-list"
      >
        <template #default="{ item }">
          <div class="upload-list-item">
            <div class="upload-item-prefix">
              <v-progress-circular
                v-if="item.status === 'uploading'"
                :model-value="(item.bytesUploaded / item.size) * 100"
                :indeterminate="item.bytesUploaded / item.size === 1"
                color="primary"
                size="24"
                width="3"
              />
              <v-icon
                v-else
                :color="getStatusColor(item.status)"
                :icon="getStatusIcon(item.status)"
              />
            </div>

            <div class="upload-item-details">
              <div class="upload-item-name">{{ item.name }}</div>
              <div class="upload-item-meta">
                {{ prettyBytes(item.bytesUploaded) }} / {{ prettyBytes(item.size) }}
                <span v-if="item.error" class="error-text">&bull; {{ item.error }}</span>
              </div>
            </div>

            <v-spacer />

            <div class="upload-item-actions">
              <span class="progress-percent" v-if="item.status === 'uploading'">
                {{ ((item.bytesUploaded / item.size) * 100).toFixed(0) }}%
              </span>
              <v-btn
                v-if="item.status === 'uploading' || item.status === 'pending'"
                color="error"
                variant="tonal"
                :icon="MdiStop"
                size="x-small"
                @click="uploadStore.stopUpload(item.id)"
              />
              <v-btn
                v-else
                color="grey"
                variant="text"
                :icon="MdiClose"
                size="x-small"
                @click="uploadStore.removeUpload(item.id)"
              />
            </div>
          </div>
        </template>
      </v-virtual-scroll>
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

.card-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 12px;
}

.is-mobile .card-title {
  font-size: 1.1rem;
}

.active-uploads-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.active-uploads-list {
  max-height: 350px;
  padding-right: 4px;
  width: 100%;
}

.upload-list-item {
  display: flex;
  align-items: center;
  padding: 10px 14px;
  background-color: rgb(var(--v-theme-surface-container-high));
  border-radius: 16px;
  margin-bottom: 6px;
  height: 56px;
  box-sizing: border-box;
  transition: background-color 0.2s ease;
  width: 100%;
}

.is-mobile .upload-list-item {
  padding: 8px 10px;
}

.upload-list-item:hover {
  background-color: rgb(var(--v-theme-surface-container-highest));
}

.upload-item-prefix {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.upload-item-details {
  margin-left: 12px;
  flex-grow: 1;
  min-width: 0;
}

.upload-item-name {
  font-weight: 600;
  font-size: 0.85rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 220px;
}

.is-mobile .upload-item-name {
  max-width: 130px;
  font-size: 0.8rem;
}

.upload-item-meta {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.is-mobile .upload-item-meta {
  font-size: 0.7rem;
}

.error-text {
  color: rgb(var(--v-theme-error));
  font-weight: 500;
}

.upload-item-actions {
  margin-left: 8px;
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.progress-percent {
  margin-right: 8px;
  font-size: 0.75rem;
  font-weight: 700;
}
</style>
