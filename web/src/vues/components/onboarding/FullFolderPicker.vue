<script setup lang="ts">
import { usePickFolderStore } from '@/scripts/stores/pickFolderStore.ts'
import FolderPicker from '@/vues/components/onboarding/FolderPicker.vue'
import ShowSelectedFolder from '@/vues/components/onboarding/ShowSelectedFolder.vue'
import MediaSample from '@/vues/components/onboarding/MediaSample.vue'

const pickFolderStore = usePickFolderStore()
</script>

<template>
  <div>
    <folder-picker class="folder-picker" />
    <div class="text-center mt-4 mb-3">
      <show-selected-folder include-selected-text pill :folder="pickFolderStore.viewedFolder" />
    </div>

    <media-sample
      class="mt-3 media-sample"
      :media-samples="pickFolderStore.mediaSamples"
      :images="pickFolderStore.samples"
    />
  </div>
</template>

<style scoped>
.media-sample {
  min-height: 430px;
}

.folder-picker {
  width: calc(100% - 10px);
  margin-left: 5px;
}
</style>
