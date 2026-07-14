<script setup lang="ts">
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import ThemeSettings from '@/vues/components/settings/ThemeSettings.vue'
import InterfaceSettings from '@/vues/components/settings/InterfaceSettings.vue'
import { DEFAULT_BG_URL, useBackgroundStore } from '@/scripts/stores/backgroundStore.ts'
import BackupSettings from '@/vues/components/settings/BackupSettings.vue'
import UploadSettings from '@/vues/components/settings/UploadSettings.vue'

const backgroundStore = useBackgroundStore()
</script>

<template>
  <main-layout-container class="settings">
    <div class="settings-content">
      <h1 class="settings-title">Settings</h1>

      <div class="divider-flex">
        <v-divider />
        <span>Theming</span>
        <v-divider />
      </div>

      <v-alert
        variant="tonal"
        type="info"
        rounded="xl"
        v-if="backgroundStore.backgroundUrl === DEFAULT_BG_URL"
        class="etna-text"
      >
        Theming with background images is not supported until at least one of your photos is fully
        processed.
      </v-alert>
      <theme-settings />

      <div class="divider-flex">
        <v-divider />
        <span>User Interface</span>
        <v-divider />
      </div>

      <interface-settings />

      <div class="divider-flex">
        <v-divider />
        <span>Application settings</span>
        <v-divider />
      </div>

      <upload-settings class="mb-6"/>

      <backup-settings />
    </div>
  </main-layout-container>
</template>

<style scoped>
.settings-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 32px 24px;
}

.etna-text {
  margin-bottom: 30px;
}

.settings-title {
  font-size: 2.125rem;
  font-weight: 700;
  margin-bottom: 32px;
  color: rgb(var(--v-theme-on-surface));
}

.divider-flex {
  display: flex;
  gap: 30px;
  margin-top: 25px;
  margin-bottom: 25px;
  margin-left: 50px;
  margin-right: 50px;
  align-items: center;
}

.divider-flex span {
  font-size: 25px;
  white-space: nowrap;
  color: rgba(var(--v-theme-on-surface-variant), 0.8);
}

.placeholder-section {
  margin-top: 32px;
  border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  padding-top: 24px;
}

.placeholder-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 8px;
}

.placeholder-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
}
</style>
