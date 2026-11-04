<script setup lang="ts">
import { shallowRef } from 'vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import MdiArchiveArrowUpOutline from '~icons/mdi/archive-arrow-up-outline'
import MdiClose from '~icons/mdi/close'
import MdiInformation from '~icons/mdi/information'

defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const uploadStore = useUploadStore()
const takeoutInput = shallowRef<HTMLInputElement | null>(null)

function triggerSelect() {
  takeoutInput.value?.click()
}

function onTakeoutChanged(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    uploadStore.addTakeoutFiles(target.files)
    emit('update:modelValue', false)
  }
  target.value = ''
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="560px"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card rounded="xl" color="surface-container-highest" class="takeout-dialog">
      <div class="dialog-header">
        <v-avatar rounded="lg" size="44" class="dialog-icon">
          <v-icon :icon="MdiArchiveArrowUpOutline" />
        </v-avatar>
        <div class="dialog-heading">
          <div class="dialog-title">Import Google Photos Takeout</div>
          <div class="dialog-subtitle">Bring an export into your photo library</div>
        </div>
        <v-btn
          :icon="MdiClose"
          variant="text"
          density="comfortable"
          rounded="xl"
          aria-label="Close dialog"
          @click="emit('update:modelValue', false)"
        />
      </div>

      <v-card-text class="dialog-content">
        <p class="dialog-intro">
          Google Takeout splits your library across multiple ZIP files. To import your photos and
          videos into Ruurd Photos you'll need to upload all of these zips at once.
        </p>

        <div class="instructions">
          <ul>
            <li>Select every ZIP file from the same Takeout export.</li>
            <li>Keep this tab open while it's uploading.</li>
            <li>
              The import begins after all selected files finish uploading. You can then close the
              tab if you wish.
            </li>
            <li>
              If an upload is interrupted, select the full set again. It will resume where it left
              off.
            </li>
          </ul>
        </div>

        <div class="storage-note">
          <v-icon :icon="MdiInformation" color="primary" size="20" />
          <div>
            <div class="storage-title">Check free space on the app_data drive</div>
            <p>
              The server uploads your takeout ZIP files to the App Data drive before extracting the
              contents to your media folder. Make sure there is enough free space to fit the entire
              export there at once.
            </p>
          </div>
        </div>
      </v-card-text>

      <v-divider />

      <v-card-actions class="dialog-actions">
        <v-spacer />
        <v-btn variant="text" rounded="xl" @click="emit('update:modelValue', false)">
          Cancel
        </v-btn>
        <v-btn
          color="primary"
          variant="tonal"
          rounded="xl"
          :prepend-icon="MdiArchiveArrowUpOutline"
          @click="triggerSelect"
        >
          Select Takeout ZIPs
        </v-btn>
      </v-card-actions>

      <input
        ref="takeoutInput"
        type="file"
        multiple
        accept=".zip"
        class="hidden-input"
        @change="onTakeoutChanged"
      />
    </v-card>
  </v-dialog>
</template>

<style scoped>
.takeout-dialog {
  border: 1px solid rgba(var(--v-border-color), 0.12) !important;
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 20px 24px;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
}

.dialog-heading {
  flex: 1;
  min-width: 0;
}

.dialog-icon {
  background-color: rgba(var(--v-theme-primary), 0.12);
}

.dialog-title {
  font-size: 1.15rem;
  font-weight: 700;
  line-height: 1.35;
}

.dialog-subtitle {
  margin-top: 2px;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 0.85rem;
}

.dialog-content {
  padding: 20px 24px 16px;
}

.dialog-intro,
.matching-note {
  line-height: 1.5;
}

.dialog-intro {
  margin-bottom: 18px;
}

.instructions {
  margin-bottom: 18px;
  padding: 14px 16px;
  border-radius: 14px;
  background-color: rgb(var(--v-theme-surface-container-low));
}

.section-label {
  margin-bottom: 8px;
  font-size: 0.8rem;
  font-weight: 700;
  color: rgba(var(--v-theme-on-surface), 0.72);
  letter-spacing: 0.04em;
}

.instructions ul {
  display: grid;
  gap: 6px;
  margin: 0;
  padding-left: 20px;
  line-height: 1.45;
}

.storage-note {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 14px 16px;
  border: 1px solid rgba(var(--v-theme-primary), 0.16);
  border-radius: 14px;
  background-color: rgba(var(--v-theme-primary), 0.045);
}

.storage-note > .v-icon {
  flex: 0 0 auto;
  margin-top: 1px;
}

.storage-title {
  margin-bottom: 3px;
  font-weight: 700;
}

.storage-note p {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.76);
  font-size: 0.875rem;
  line-height: 1.5;
}

.storage-note code {
  padding: 1px 4px;
  border-radius: 4px;
  background-color: rgba(var(--v-theme-on-surface), 0.08);
  font-size: 0.85em;
}

.matching-note {
  margin: 16px 0 0;
  color: rgba(var(--v-theme-on-surface), 0.7);
  font-size: 0.875rem;
}

.dialog-actions {
  gap: 8px;
  padding: 16px 24px 20px;
}

.is-mobile .dialog-header {
  gap: 10px;
  padding: 16px;
}

.is-mobile .dialog-title {
  font-size: 1rem;
}

.is-mobile .dialog-subtitle {
  font-size: 0.8rem;
}

.is-mobile .dialog-content {
  padding: 16px 16px 12px;
}

.is-mobile .instructions,
.is-mobile .storage-note {
  padding: 12px;
}

.is-mobile .dialog-actions {
  padding: 12px 16px 16px;
  flex-wrap: wrap;
}

.hidden-input {
  display: none;
}
</style>
