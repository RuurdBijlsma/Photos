<script setup lang="ts">
import { UPLOAD_CONCURRENCY_LIMIT, useSettingStore } from '@/scripts/stores/settingsStore.ts'
import SettingsSlider from '@/vues/components/settings/components/SettingsSlider.vue'

const settings = useSettingStore()
</script>

<template>
  <div class="theme-settings-layout">
    <!-- Settings Configuration Panel -->
    <section class="config-panel">
      <v-card class="settings-card" flat border>
        <!-- Card Header -->
        <div class="card-header">
          <span class="card-title">Upload Config</span>
          <v-icon color="primary" size="large">mdi-cog-outline</v-icon>
        </div>

        <div class="card-body">
          <!-- Section: Mode Settings -->
          <div class="section-divider">
            <span class="section-label">Uploads</span>
            <v-divider class="divider-line" />
          </div>

          <!-- Section: Contrast Settings -->
          <settings-slider
            v-model="settings.uploadConcurrencyLimit"
            label="Concurrent Upload Connections:"
            :min="1"
            :max="10"
            :step="1"
            :reset-value="UPLOAD_CONCURRENCY_LIMIT"
            description="Increase or decrease simultaneous uploads to balance connection performance."
          />
        </div>
      </v-card>
    </section>
  </div>
</template>

<style scoped>
.theme-settings-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 24px;
}

@media (min-width: 1280px) {
  .theme-settings-layout {
    grid-template-columns: 7fr 5fr;
  }
}

.settings-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 24px !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.height-100 {
  height: 100%;
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

.section-divider {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
}

.color-section-divider {
  margin-top: 24px;
}

.section-label {
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-primary));
}

.divider-line {
  margin-left: 16px;
  opacity: 0.3;
}

.chip-group-wrapper {
  margin-bottom: 24px;
}

.theme-chip {
  padding: 16px 20px !important;
}

:deep(.v-chip--selected) {
  font-weight: 600;
}

.schedule-settings {
  margin-bottom: 24px;
  background-color: rgb(var(--v-theme-surface-container-highest));
  padding: 20px;
  border-radius: 8px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.schedule-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
  display: block;
  margin-bottom: 8px;
}

.schedule-switch {
  margin-bottom: 16px;
}

.time-picker-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
  margin-top: 16px;
}

.time-picker-card {
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
  border-radius: 16px !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.time-picker-header {
  padding: 12px 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.time-picker-label {
  font-size: 0.75rem;
  font-weight: 700;
}

.time-picker-body {
  padding: 12px;
  display: flex;
  justify-content: center;
}

.color-settings-content {
  margin-bottom: 24px;
}

.background-switch {
  margin-bottom: 16px;
}

.new-background-btn {
  margin-bottom: 16px;
}

.color-picker-wrapper {
  margin-bottom: 24px;
}

.color-picker-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
  display: block;
  margin-bottom: 12px;
}

.color-picker-card {
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
  display: inline-block !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
  padding: 8px;
  border-radius: 24px !important;
}

.variant-settings {
  margin-bottom: 16px;
}

.variant-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
  display: block;
  margin-bottom: 8px;
}

.visualizer-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 16px;
}

.swatch-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 12px;
}

.swatch-card {
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease;
  min-height: 100px;
  border-radius: 8px !important;
  border: 1px solid rgba(var(--v-border-color), 0.1) !important;
}

.swatch-card:hover {
  transform: translateY(-2px);
}

.swatch-content {
  padding: 12px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  height: 100%;
}

.swatch-header {
  display: flex;
  flex-direction: column;
}

.swatch-name {
  font-size: 1rem;
  font-weight: 900;
  line-height: 1.2;
}

.swatch-class-label {
  font-family: monospace;
  font-size: 0.85rem;
  margin-top: 4px;
  text-transform: lowercase;
}

.swatch-desc-container {
  text-align: right;
  font-size: 0.72rem;
  margin-top: 12px;
  padding-top: 8px;
  border-top: 1px solid rgba(var(--v-border-color), 0.1);
}
</style>
