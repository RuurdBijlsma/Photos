<script setup lang="ts">
import { ref } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import ShowSelectedFolder from '@/vues/components/onboarding/ShowSelectedFolder.vue'
import MdiFolderSearchOutline from '~icons/mdi/folder-search-outline'

const ingestStore = useIngestJobsStore()
const authStore = useAuthStore()

const isScanning = ref(false)

async function handleScan() {
  isScanning.value = true
  try {
    await ingestStore.triggerScan()
  } catch {
    // Managed in store
  } finally {
    isScanning.value = false
  }
}
</script>

<template>
  <v-card class="action-card" flat>
    <div class="action-content">
      <div class="action-text">
        <div class="action-description">
          <h2 class="section-title">Index Library Folder</h2>
          <p class="section-subtitle">
            Folder: Start a search of your media folder to discover new photos and videos.
          </p>
        </div>
        <show-selected-folder
          class="selected-folder-display"
          bg-color="surface-variant"
          text-color="on-surface-variant"
          exclude-check-icon
          v-if="authStore.user?.mediaFolder"
          :folder="authStore.user.mediaFolder.split('/')"
          pill
        />
      </div>
      <v-btn
        color="secondary"
        variant="tonal"
        rounded="xl"
        :prepend-icon="MdiFolderSearchOutline"
        :loading="isScanning"
        @click="handleScan"
        class="scan-button"
      >
        Scan Folder
      </v-btn>
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

.action-content {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 20px;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
}

.is-mobile .action-content {
  padding: 16px 14px;
}

.action-text {
  display: flex;
  align-items: flex-start;
  width: 100%;
  max-width: 100%;
  min-width: 0;
}

.is-mobile .action-text {
  flex-direction: column;
  gap: 12px;
}

.action-description {
  flex-grow: 1;
  min-width: 0;
  max-width: 100%;
}

.section-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
}

.is-mobile .section-title {
  font-size: 1.1rem;
}

.section-subtitle {
  font-size: 0.875rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
  margin-top: 4px;
  margin-bottom: 0;
}

.is-mobile .section-subtitle {
  font-size: 0.8rem;
}

.selected-folder-display {
  margin-top: 12px;
  margin-right: 4px;
  max-width: 100%;
  overflow: hidden;
}

.is-mobile .selected-folder-display {
  margin-top: 0;
  margin-right: 0;
}

.scan-button {
  margin-top: 16px;
}

.is-mobile .scan-button {
  width: 100%;
}
</style>
