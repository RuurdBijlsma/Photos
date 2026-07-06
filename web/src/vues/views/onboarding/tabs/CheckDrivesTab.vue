<script setup lang="ts">
import FullFolderStatus from '@/vues/components/onboarding/FullFolderStatus.vue'
import { ref } from 'vue'
import { useAdminStore } from '@/scripts/stores/adminStore.ts'

const adminStore = useAdminStore()
const refreshLoading = ref(false)

async function refreshFolderSummary() {
  refreshLoading.value = true
  await adminStore.fetchDiskInfo()
  refreshLoading.value = false
}

refreshFolderSummary().then()
</script>

<template>
  <!-- Folders Status Section -->
  <div class="folder-status-title mb-3">
    <p class="text-caption text-medium-emphasis text-primary">
      Make sure the correct drives are linked before starting the indexing process.
    </p>
    <v-spacer />
    <v-btn
      color="primary"
      variant="plain"
      density="compact"
      @click="refreshFolderSummary"
      :loading="refreshLoading"
      prepend-icon="mdi-refresh"
      >Refresh
    </v-btn>
  </div>
  <section v-if="adminStore.disks">
    <!-- Media Folder -->
    <full-folder-status
      :folder="adminStore.disks.mediaFolder"
      env-var="MEDIA_DIR"
      title-icon="mdi-camera"
    />

    <!-- Thumbnails Folder -->
    <full-folder-status
      :folder="adminStore.disks.appDataFolder"
      env-var="THUMBNAILS_DIR"
      title-icon="mdi-image-multiple"
    />
  </section>

  <v-skeleton-loader
    :loading="refreshLoading"
    v-else
    type="card-avatar, heading, paragraph"
  ></v-skeleton-loader>
</template>

<style scoped>
.folder-status-title {
  display: flex;
  align-items: center;
}
</style>
