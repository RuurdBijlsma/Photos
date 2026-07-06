<script setup lang="ts">
import UnsupportedFiles from '@/vues/components/onboarding/UnsupportedFiles.vue'
import InaccessibleEntries from '@/vues/components/onboarding/InaccessibleEntries.vue'
import ShowSelectedFolder from '@/vues/components/onboarding/ShowSelectedFolder.vue'
import { usePickFolderStore } from '@/scripts/stores/pickFolderStore.ts'

const pickFolderStore = usePickFolderStore()
</script>

<template>
  <v-card variant="text" color="primary" class="top-bar mb-5">
    <show-selected-folder
      icon-color="on-surface-variant"
      :pill="true"
      color="on_surface"
      :folder="pickFolderStore.viewedFolder"
    />
    <v-spacer />
    <v-btn
      :loading="pickFolderStore.unsupportedFilesLoading"
      prepend-icon="mdi-refresh"
      rounded
      variant="text"
      @click="pickFolderStore.refreshUnsupportedFiles"
      color="primary"
      >Refresh
    </v-btn>
  </v-card>

  <div
    v-if="
      pickFolderStore.unsupportedFiles &&
      (pickFolderStore.unsupportedFiles.unsupportedCount > 0 ||
        pickFolderStore.unsupportedFiles.inaccessibleEntries.length > 0)
    "
  >
    <!-- Unsupported Files -->
    <unsupported-files
      v-if="pickFolderStore.unsupportedFiles.unsupportedCount > 0"
      :summary="pickFolderStore.unsupportedFiles"
    />

    <!-- Inaccessible Files and Folders -->
    <inaccessible-entries
      :summary="pickFolderStore.unsupportedFiles"
      v-if="pickFolderStore.unsupportedFiles.inaccessibleEntries.length > 0"
    />
  </div>
  <v-alert
    variant="flat"
    color="surface-container"
    v-else-if="pickFolderStore.unsupportedFiles"
    class="rounded-xl text-md-caption"
    icon="mdi-check"
  >
    <p :style="{ color: 'rgb(var(--v-theme-on-surface-container))' }">
      Great news! There are no unsupported files in your selection. Everything looks good!
    </p>
  </v-alert>
</template>

<style scoped>
.top-bar {
  display: flex;
  align-items: center;
}

.pill {
  padding: 24px 32px;
  border-radius: 32px;
}
</style>
