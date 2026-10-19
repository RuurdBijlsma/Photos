<script setup lang="ts">
import { ref, shallowRef } from 'vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import TakeoutImportDialog from '@/vues/components/activity/cards/TakeoutImportDialog.vue'
import MdiArchiveArrowUpOutline from '~icons/mdi/archive-arrow-up-outline'
import MdiFolderOpen from '~icons/mdi/folder-open'
import MdiTrashCanOutline from '~icons/mdi/trash-can-outline'
import MdiTrayArrowUp from '~icons/mdi/tray-arrow-up'

const uploadStore = useUploadStore()

const fileInput = shallowRef<HTMLInputElement | null>(null)
const folderInput = shallowRef<HTMLInputElement | null>(null)
const takeoutDialog = ref(false)
const dragover = ref(false)

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
  if (e.dataTransfer?.files?.length) {
    uploadStore.addFiles(e.dataTransfer.files)
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
</script>

<template>
  <v-card class="action-card" flat>
    <div class="card-body">
      <h2 class="card-title">Upload Media</h2>
      <div
        class="custom-dropzone"
        :class="{ dragover: dragover }"
        @dragover.prevent="onDragOver"
        @dragleave="onDragLeave"
        @drop.prevent="onDrop"
        @click="triggerFileSelect"
      >
        <v-icon size="x-large" color="primary" class="dropzone-icon" :icon="MdiTrayArrowUp" />
        <div class="dropzone-label">
          Drag & Drop files here or <span class="browse-link">Browse</span>
        </div>
        <div class="dropzone-hint">Supports image and video file formats</div>
      </div>

      <input ref="fileInput" type="file" multiple class="hidden-input" @change="onFileChanged" />
      <input
        ref="folderInput"
        type="file"
        multiple
        webkitdirectory
        directory
        class="hidden-input"
        @change="onFolderChanged"
      />

      <div class="dropzone-buttons">
        <v-btn
          variant="tonal"
          color="secondary"
          rounded
          :prepend-icon="MdiFolderOpen"
          @click="triggerFolderSelect"
        >
          Upload Folder
        </v-btn>
        <v-btn
          variant="text"
          size="small"
          :prepend-icon="MdiArchiveArrowUpOutline"
          @click="takeoutDialog = true"
        >
          Import Google Photos Takeout
        </v-btn>
        <v-btn
          v-if="uploadStore.uploads.length > 0"
          variant="plain"
          color="error"
          rounded
          :prepend-icon="MdiTrashCanOutline"
          @click="uploadStore.clearCompleted"
          class="clear-button"
        >
          Clear Completed
        </v-btn>
      </div>

      <TakeoutImportDialog v-model="takeoutDialog" />
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

.custom-dropzone {
  border: 2px dashed rgba(var(--v-theme-primary), 0.4);
  background-color: rgba(var(--v-theme-primary), 0.02);
  border-radius: 20px;
  padding: 36px 20px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s ease-in-out;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
}

.is-mobile .custom-dropzone {
  padding: 22px 14px;
  border-radius: 16px;
}

.custom-dropzone:hover,
.custom-dropzone.dragover {
  border-color: rgb(var(--v-theme-primary));
  background-color: rgba(var(--v-theme-primary), 0.06);
}

.dropzone-icon {
  margin-bottom: 8px;
}

.dropzone-label {
  font-size: 0.95rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.is-mobile .dropzone-label {
  font-size: 0.88rem;
}

.browse-link {
  color: rgb(var(--v-theme-primary));
  text-decoration: underline;
  font-weight: 700;
}

.dropzone-hint {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
  margin-top: 4px;
}

.hidden-input {
  display: none;
}

.dropzone-buttons {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 16px;
  width: 100%;
}

.is-mobile .dropzone-buttons {
  width: 100%;
}

.clear-button {
  margin-left: 8px;
}

.is-mobile .clear-button {
  margin-left: 0;
}
</style>
