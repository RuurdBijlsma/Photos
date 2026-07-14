<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { prettyBytes } from '@/scripts/utils.ts'

const uploadStore = useUploadStore()

const fileInput = ref<HTMLInputElement | null>(null)
const folderInput = ref<HTMLInputElement | null>(null)
const dragover = ref(false)

const successCount = computed(
  () => uploadStore.uploads.filter((u) => u.status === 'success').length,
)
const failedCount = computed(() => uploadStore.uploads.filter((u) => u.status === 'failed').length)

function triggerFileSelect() {
  fileInput.value?.click()
}

function triggerFolderSelect() {
  folderInput.value?.click()
}

function onDragOver() {
  dragover.value = true
}

function onDragLeave() {
  dragover.value = false
}

function onDrop(e: DragEvent) {
  dragover.value = false
  if (!e.dataTransfer) return

  const filesToUpload: File[] = []

  if (e.dataTransfer.items && e.dataTransfer.items.length > 0) {
    for (const item of Array.from(e.dataTransfer.items)) {
      if (item.kind !== 'file') continue

      const entry = typeof item.webkitGetAsEntry === 'function' ? item.webkitGetAsEntry() : null

      if (entry) {
        if (entry.isFile) {
          const file = item.getAsFile()
          if (file) filesToUpload.push(file)
        } else if (entry.isDirectory) {
          console.warn(
            `Dropped directory "${entry.name}" was ignored. Please use the "Upload Folder" button.`,
          )
        }
      } else {
        const file = item.getAsFile()
        if (file) filesToUpload.push(file)
      }
    }
  } else if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
    for (const file of Array.from(e.dataTransfer.files)) {
      filesToUpload.push(file)
    }
  }

  if (filesToUpload.length > 0) {
    uploadStore.addFiles(filesToUpload)
  }
}

function onFileChanged(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    uploadStore.addFiles(target.files)
  }
}

function onFolderChanged(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    uploadStore.addFiles(target.files)
  }
}

function getStatusColor(status: string) {
  switch (status) {
    case 'uploading':
      return 'primary'
    case 'success':
      return 'success'
    case 'failed':
      return 'error'
    case 'stopped':
      return 'warning'
    default:
      return 'grey-lighten-1'
  }
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'uploading':
      return 'mdi-cloud-upload-outline'
    case 'success':
      return 'mdi-check-circle-outline'
    case 'failed':
      return 'mdi-alert-circle-outline'
    case 'stopped':
      return 'mdi-pause-circle-outline'
    default:
      return 'mdi-clock-outline'
  }
}

onBeforeRouteLeave((to, from, next) => {
  if (uploadStore.isUploading) {
    const confirmation = window.confirm(
      'Uploads are currently in progress. Navigating away may disrupt the process. Do you still want to leave?',
    )
    if (confirmation) {
      next()
    } else {
      next(false)
    }
  } else {
    next()
  }
})

function handleBeforeUnload(e: BeforeUnloadEvent) {
  if (uploadStore.isUploading) {
    e.preventDefault()
    e.returnValue = ''
  }
}

onMounted(() => {
  window.addEventListener('beforeunload', handleBeforeUnload)
})

onUnmounted(() => {
  window.removeEventListener('beforeunload', handleBeforeUnload)
})
</script>

<template>
  <main-layout-container class="upload-scroll-view">
    <div class="upload-content">
      <header class="upload-header mb-6">
        <h1 class="upload-title">Upload Media</h1>
        <p class="upload-subtitle">Import local photos and videos into your library dashboard.</p>
      </header>

      <div class="upload-grid">
        <section class="picker-panel">
          <v-card class="settings-card" flat border>
            <div class="card-header">
              <span class="card-title">Select Media</span>
              <v-icon color="primary" size="large">mdi-cloud-upload</v-icon>
            </div>

            <div class="card-body">
              <div
                class="dropzone"
                :class="{ dragover: dragover }"
                @dragover.prevent="onDragOver"
                @dragleave="onDragLeave"
                @drop.prevent="onDrop"
                @click="triggerFileSelect"
              >
                <v-icon size="x-large" color="primary" class="mb-3">mdi-tray-arrow-up</v-icon>
                <div class="dropzone-text">
                  Drag & Drop files here or <span class="browse-link">Browse</span>
                </div>
                <div class="dropzone-subtext mt-1">Supports common image and video formats</div>
              </div>

              <input ref="fileInput" type="file" multiple class="d-none" @change="onFileChanged" />

              <input
                ref="folderInput"
                type="file"
                multiple
                webkitdirectory
                directory
                class="d-none"
                @change="onFolderChanged"
              />

              <div class="d-flex align-center gap-4 mt-4 justify-center flex-wrap">
                <v-btn
                  variant="tonal"
                  color="secondary"
                  rounded
                  prepend-icon="mdi-folder-open"
                  @click="triggerFolderSelect"
                >
                  Upload Folder
                </v-btn>
                <v-btn
                  v-if="uploadStore.uploads.length > 0"
                  variant="plain"
                  color="error"
                  rounded
                  prepend-icon="mdi-trash-can-outline"
                  @click="uploadStore.clearCompleted"
                >
                  Clear Completed
                </v-btn>
              </div>

              <v-expand-transition>
                <div v-if="uploadStore.uploads.length > 0" class="stats-overview mt-6">
                  <div class="section-divider">
                    <span class="section-label">Queue Summary</span>
                    <v-divider class="divider-line" />
                  </div>

                  <div class="stats-grid">
                    <div class="stat-item">
                      <span class="stat-value">{{ uploadStore.uploads.length }}</span>
                      <span class="stat-label">Total</span>
                    </div>
                    <div class="stat-item">
                      <span class="stat-value text-primary">{{ uploadStore.activeCount }}</span>
                      <span class="stat-label">Active</span>
                    </div>
                    <div class="stat-item">
                      <span class="stat-value text-success">{{ successCount }}</span>
                      <span class="stat-label">Done</span>
                    </div>
                    <div class="stat-item">
                      <span class="stat-value text-error">{{ failedCount }}</span>
                      <span class="stat-label">Failed</span>
                    </div>
                  </div>

                  <v-btn
                    block
                    variant="tonal"
                    color="error"
                    prepend-icon="mdi-stop-circle-outline"
                    class="mt-4"
                    rounded
                    @click="uploadStore.abortAll"
                  >
                    Cancel All Operations
                  </v-btn>
                </div>
              </v-expand-transition>
            </div>
          </v-card>
        </section>

        <section class="queue-panel">
          <v-card class="settings-card height-100" flat border>
            <div class="card-header">
              <span class="card-title">Upload Progress</span>
              <v-icon color="secondary" size="large">mdi-format-list-bulleted</v-icon>
            </div>

            <div class="card-body px-0 py-2">
              <v-virtual-scroll
                v-if="uploadStore.uploads.length > 0"
                :items="uploadStore.uploads"
                height="500"
                class="queue-virtual-scroll"
              >
                <template v-slot:default="{ item }">
                  <v-list-item class="queue-item" border>
                    <!-- Prepend slot: Dynamically renders either active circular progress or status icon -->
                    <template v-slot:prepend>
                      <div class="status-icon-container">
                        <v-progress-circular
                          v-if="item.status === 'uploading'"
                          :model-value="(item.bytesUploaded / item.size) * 100"
                          :indeterminate="item.bytesUploaded / item.size === 1"
                          color="primary"
                          size="24"
                          width="3"
                        />
                        <v-icon v-else :color="getStatusColor(item.status)">
                          {{ getStatusIcon(item.status) }}
                        </v-icon>
                      </div>
                    </template>

                    <v-list-item-title class="queue-filename">
                      {{ item.name }}
                    </v-list-item-title>
                    <v-list-item-subtitle class="queue-meta">
                      {{ prettyBytes(item.bytesUploaded) }} / {{ prettyBytes(item.size) }}
                      <span v-if="item.error" class="text-error ml-2">&bull; {{ item.error }}</span>
                    </v-list-item-subtitle>

                    <template v-slot:append>
                      <div class="d-flex align-center gap-2">
                        <span class="progress-percent mr-2" v-if="item.status === 'uploading'">
                          {{ ((item.bytesUploaded / item.size) * 100).toFixed(0) }}%
                        </span>

                        <v-btn
                          v-if="item.status === 'uploading' || item.status === 'pending'"
                          color="error"
                          variant="tonal"
                          icon="mdi-stop"
                          size="small"
                          flat
                          @click="uploadStore.stopUpload(item.id)"
                        />
                        <v-btn
                          v-else
                          color="grey"
                          variant="text"
                          icon="mdi-close"
                          size="small"
                          @click="uploadStore.removeUpload(item.id)"
                        />
                      </div>
                    </template>
                  </v-list-item>
                </template>
              </v-virtual-scroll>

              <div
                v-else
                class="empty-queue-placeholder d-flex flex-column align-center justify-center py-12"
              >
                <v-icon size="x-large" color="disabled" class="mb-2 opacity-50"
                  >mdi-cloud-upload-outline</v-icon
                >
                <div class="text-disabled text-center">No active upload operations are queued.</div>
              </div>
            </div>
          </v-card>
        </section>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.upload-scroll-view {
  overflow-y: auto;
}

.upload-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 32px 24px;
}

.upload-title {
  font-size: 2.125rem;
  font-weight: 700;
  margin-bottom: 6px;
  color: rgb(var(--v-theme-on-surface));
}

.upload-subtitle {
  font-size: 0.95rem;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 0;
}

.upload-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 24px;
}

@media (min-width: 1280px) {
  .upload-grid {
    grid-template-columns: 5fr 7fr;
  }
}

.settings-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 24px !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.card-header {
  background-color: rgb(var(--v-theme-surface-container-high));
  padding: 16px 24px;
  border-top-left-radius: 24px;
  border-top-right-radius: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.card-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.card-body {
  padding: 24px;
}

.dropzone {
  border: 2px dashed rgba(var(--v-theme-primary), 0.4);
  background-color: rgba(var(--v-theme-primary), 0.02);
  border-radius: 16px;
  padding: 48px 24px;
  text-align: center;
  cursor: pointer;
  transition:
    background-color 0.2s ease,
    border-color 0.2s ease;
}

.dropzone:hover,
.dropzone.dragover {
  border-color: rgb(var(--v-theme-primary));
  background-color: rgba(var(--v-theme-primary), 0.06);
}

.dropzone-text {
  font-size: 1rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.browse-link {
  color: rgb(var(--v-theme-primary));
  text-decoration: underline;
  font-weight: 600;
}

.dropzone-subtext {
  font-size: 0.8rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.section-divider {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
}

.section-label {
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-primary));
  white-space: nowrap;
}

.divider-line {
  margin-left: 16px;
  opacity: 0.3;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 16px;
}

.stat-item {
  background-color: rgb(var(--v-theme-surface-container-high));
  border-radius: 12px;
  padding: 12px 6px;
  text-align: center;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.stat-value {
  font-size: 1.25rem;
  font-weight: 700;
  display: block;
}

.stat-label {
  font-size: 0.7rem;
  color: rgb(var(--v-theme-on-surface-variant));
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.queue-virtual-scroll {
  background: transparent;
}

.status-icon-container {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  margin-right: 16px;
}

.queue-item {
  padding: 14px 16px !important;
  background-color: transparent !important;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
  position: relative;
}

.queue-filename {
  font-weight: 600;
  font-size: 0.95rem !important;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.queue-meta {
  font-size: 0.8rem !important;
  opacity: 0.8;
}

.progress-percent {
  font-weight: 700;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-primary));
}

.height-100 {
  height: 100%;
}

.gap-4 {
  gap: 16px;
}

.empty-queue-placeholder {
  min-height: 300px;
}
</style>
