<script setup lang="ts">
import { shallowRef } from 'vue'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import MdiArchiveArrowUpOutline from '~icons/mdi/archive-arrow-up-outline'

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
    <v-card rounded="xl" color="surface-container-highest">
      <v-card-title class="dialog-title">Import Google Photos Takeout</v-card-title>

      <v-card-text class="dialog-text">
        <p>
          Google Takeout splits your library over many ZIP files, and the photos of one album can be
          spread across several of them.
        </p>
        <p>
          <strong>Upload all ZIP files from the same export at once.</strong> The import starts when
          the last ZIP has finished uploading.
        </p>
        <p>
          Photos that are already in your library are not imported again; they are only added to the
          matching albums. Upload progress is shown on this page.
        </p>
      </v-card-text>

      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="emit('update:modelValue', false)">Cancel</v-btn>
        <v-btn
          color="primary"
          variant="tonal"
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
.dialog-title {
  padding-bottom: 4px;
}

.dialog-text p {
  margin-bottom: 12px;
  line-height: 1.5;
}

.dialog-text p:last-child {
  margin-bottom: 0;
}

.hidden-input {
  display: none;
}
</style>
