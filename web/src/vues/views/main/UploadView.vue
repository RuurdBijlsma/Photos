<script setup lang="ts">
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { ref } from 'vue'
import { prettyBytes } from '@/scripts/utils.ts'

const uploadStore = useUploadStore()
const fileInput = ref<HTMLInputElement | null>(null)
const isDragging = ref(false)

function triggerFileInput() {
  fileInput.value?.click()
}

function onFileSelected(event: Event) {
  const target = event.target as HTMLInputElement
  if (target.files) {
    const files = Array.from(target.files)
    uploadStore.addFiles(files)
  }
}

function onDragOver(event: DragEvent) {
  event.preventDefault()
  isDragging.value = true
}

function onDragLeave() {
  isDragging.value = false
}

function onDrop(event: DragEvent) {
  event.preventDefault()
  isDragging.value = false
  if (event.dataTransfer?.files) {
    const files = Array.from(event.dataTransfer.files)
    uploadStore.addFiles(files)
  }
}

function getStatusColor(status: string) {
  switch (status) {
    case 'uploading':
      return 'primary'
    case 'completed':
      return 'success'
    case 'failed':
      return 'error'
    default:
      return 'grey'
  }
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'uploading':
      return 'mdi-upload'
    case 'completed':
      return 'mdi-check-circle'
    case 'failed':
      return 'mdi-alert-circle'
    default:
      return 'mdi-clock-outline'
  }
}
</script>

<template>
  <main-layout-container>
    <div class="upload-container">
      <header class="upload-header">
        <h1>Resumable Upload</h1>
        <p class="subtitle">
          Securely transfer high-definition photos and videos using the resumable TUS protocol.
        </p>
      </header>

      <!-- Interactive Drag & Drop Area -->
      <div
        class="drop-zone"
        :class="{ dragging: isDragging }"
        @dragover="onDragOver"
        @dragleave="onDragLeave"
        @drop="onDrop"
        @click="triggerFileInput"
      >
        <input
          ref="fileInput"
          type="file"
          multiple
          accept="image/*,video/*"
          class="hidden-input"
          @change="onFileSelected"
        />
        <v-icon size="80" color="primary" class="mb-4">mdi-cloud-upload-outline</v-icon>
        <h3>Drag and drop files here</h3>
        <p class="text-subtitle-2 text-disabled mt-1">
          or click to choose files from device storage
        </p>
        <span class="file-limits mt-4 text-caption text-disabled">
          Interrupted connections will resume automatically from where they left off without
          restarting.
        </span>
      </div>

      <!-- Uploading Queues -->
      <div v-if="uploadStore.uploadQueue.length > 0" class="queue-section mt-8">
        <div class="queue-header d-flex align-center justify-space-between mb-4">
          <div class="d-flex align-center gap-2">
            <h2>Transfers</h2>
            <v-chip size="small" color="primary" variant="flat">
              {{ uploadStore.uploadQueue.length }} active
            </v-chip>
          </div>
          <v-btn
            color="error"
            variant="tonal"
            prepend-icon="mdi-delete-sweep-outline"
            size="small"
            rounded
            @click="uploadStore.clearQueue"
          >
            Clear Queue
          </v-btn>
        </div>

        <v-card class="queue-card" border flat>
          <v-list bg-color="transparent" class="pa-0">
            <v-list-item
              v-for="item in uploadStore.uploadQueue"
              :key="item.id"
              class="queue-item py-3 px-4 border-bottom"
            >
              <template v-slot:prepend>
                <div class="file-icon-wrapper mr-4">
                  <v-icon
                    size="36"
                    :color="item.file.type.startsWith('video/') ? 'secondary' : 'primary'"
                  >
                    {{
                      item.file.type.startsWith('video/')
                        ? 'mdi-video-outline'
                        : 'mdi-image-outline'
                    }}
                  </v-icon>
                </div>
              </template>

              <v-list-item-title class="file-name font-weight-medium">
                {{ item.file.name }}
              </v-list-item-title>

              <v-list-item-subtitle class="file-meta mt-1 d-flex align-center gap-3">
                <span>{{ prettyBytes(item.file.size) }}</span>
                <v-chip
                  size="x-small"
                  :color="getStatusColor(item.status)"
                  variant="flat"
                  class="text-uppercase font-weight-bold"
                >
                  <v-icon start size="12" :icon="getStatusIcon(item.status)" />
                  {{ item.status }}
                </v-chip>
              </v-list-item-subtitle>

              <!-- Uploading Segment progress -->
              <div v-if="item.status === 'uploading'" class="progress-container mt-2">
                <v-progress-linear
                  v-model="item.progress"
                  color="primary"
                  height="6"
                  rounded
                  striped
                />
                <div
                  class="progress-text d-flex justify-space-between text-caption text-disabled mt-1"
                >
                  <span>Transferring resumably...</span>
                  <span>{{ item.progress }}%</span>
                </div>
              </div>

              <!-- Fail/Retry errors -->
              <div
                v-if="item.status === 'failed'"
                class="error-container mt-2 text-caption text-error"
              >
                <v-icon start size="14" icon="mdi-alert-circle-outline" />
                Connection interrupted: {{ item.error || 'Server error' }}
              </div>

              <template v-slot:append>
                <div class="item-actions d-flex align-center gap-2">
                  <v-btn
                    v-if="item.status === 'uploading'"
                    icon="mdi-pause"
                    variant="tonal"
                    color="warning"
                    density="comfortable"
                    v-tooltip="'Pause Transfer'"
                    @click="uploadStore.pauseUpload(item)"
                  />
                  <v-btn
                    v-if="item.status === 'idle' && item.progress > 0"
                    icon="mdi-play"
                    variant="tonal"
                    color="primary"
                    density="comfortable"
                    v-tooltip="'Resume Transfer'"
                    @click="uploadStore.resumeUpload(item)"
                  />
                  <v-btn
                    v-if="item.status === 'failed'"
                    icon="mdi-refresh"
                    variant="tonal"
                    color="primary"
                    density="comfortable"
                    v-tooltip="'Retry Transfer'"
                    @click="uploadStore.resumeUpload(item)"
                  />
                </div>
              </template>
            </v-list-item>
          </v-list>
        </v-card>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.upload-container {
  padding: 30px;
}

.upload-header {
  margin-bottom: 30px;
}

.upload-header h1 {
  font-size: 2.5rem;
  font-weight: 600;
  line-height: 1.2;
}

.subtitle {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 1rem;
  margin-top: 6px;
}

.drop-zone {
  border: 2px dashed rgba(var(--v-theme-primary), 0.3);
  border-radius: 24px;
  padding: 50px 30px;
  text-align: center;
  cursor: pointer;
  background-color: rgba(var(--v-theme-surface-container-low), 0.4);
  transition: all 0.2s ease-in-out;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.drop-zone:hover,
.drop-zone.dragging {
  border-color: rgb(var(--v-theme-primary));
  background-color: rgba(var(--v-theme-primary), 0.05);
}

.hidden-input {
  display: none;
}

.file-limits {
  max-width: 420px;
}

.gap-2 {
  gap: 8px;
}

.gap-3 {
  gap: 12px;
}

.border-bottom {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.08);
}

.border-bottom:last-child {
  border-bottom: none;
}

.queue-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 16px !important;
  overflow: hidden;
}

.file-icon-wrapper {
  background-color: rgba(var(--v-theme-on-surface), 0.04);
  padding: 8px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.file-name {
  font-size: 1rem;
  color: rgb(var(--v-theme-on-surface));
}

.progress-container {
  max-width: 500px;
}
</style>
