<script setup lang="ts">
import { CUSTOM_THEME_CONTRAST, useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useSunStore } from '@/scripts/stores/sunStore.ts'
import { useBackgroundStore } from '@/scripts/stores/backgroundStore.ts'
import { computed } from 'vue'
import { themeOptions, themeVariantOptions } from '@/scripts/constants.ts'
import { caps } from '@/scripts/utils.ts'
import SettingsSlider from '@/vues/components/settings/components/SettingsSlider.vue'

const settings = useSettingStore()
const sun = useSunStore()
const backgroundStore = useBackgroundStore()

const sunString = computed(() => {
  if (!sun.sunset || !sun.sunrise) {
    return ''
  }
  return ` (${sun.sunrise.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} - ${sun.sunset.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })})`
})

// Swatches to visualize semantic theme colors mapped to current theme formulas
const previewSwatches = [
  { name: 'Primary', bg: 'bg-primary', text: 'text-on-primary', desc: 'Main accent' },
  {
    name: 'Primary Container',
    bg: 'bg-primary-container',
    text: 'text-on-primary-container',
    desc: 'Primary container',
  },
  {
    name: 'Secondary',
    bg: 'bg-secondary',
    text: 'text-on-secondary',
    desc: 'Less prominent accent',
  },
  {
    name: 'Secondary Container',
    bg: 'bg-secondary-container',
    text: 'text-on-secondary-container',
    desc: 'Secondary container',
  },
  { name: 'Tertiary', bg: 'bg-tertiary', text: 'text-on-tertiary', desc: 'Contrasting accent' },
  {
    name: 'Tertiary Container',
    bg: 'bg-tertiary-container',
    text: 'text-on-tertiary-container',
    desc: 'Tertiary container',
  },
  {
    name: 'Surface Low',
    bg: 'bg-surface-container-low',
    text: 'text-on-surface',
    desc: 'Lowest emphasis card background',
  },
  {
    name: 'Surface High',
    bg: 'bg-surface-container-high',
    text: 'text-on-surface',
    desc: 'Highest emphasis card background',
  },
]
</script>

<template>
  <div class="theme-settings-layout">
    <!-- Settings Configuration Panel -->
    <section class="config-panel">
      <v-card class="settings-card" flat border>
        <!-- Card Header -->
        <div class="card-header">
          <span class="card-title">Theme Configuration</span>
          <v-icon color="primary" size="large">mdi-palette-outline</v-icon>
        </div>

        <div class="card-body">
          <!-- Section: Mode Settings -->
          <div class="section-divider">
            <span class="section-label">Mode</span>
            <v-divider class="divider-line" />
          </div>

          <div class="chip-group-wrapper">
            <v-chip-group v-model="settings.themeString" color="primary" mandatory>
              <v-chip
                v-for="opt in themeOptions"
                :value="opt"
                :key="opt"
                variant="flat"
                class="theme-chip"
              >
                {{ caps(opt) }}
              </v-chip>
            </v-chip-group>
          </div>

          <!-- Subsection: Schedule Options -->
          <v-expand-transition>
            <div v-if="settings.themeString === 'schedule'" class="schedule-settings">
              <span class="schedule-title">Schedule Theme Settings</span>
              <v-switch
                v-model="settings.useSunSchedule"
                :label="`Sunrise to sunset${sunString}`"
                hide-details
                color="primary"
                inset
                density="comfortable"
                class="schedule-switch"
              />

              <v-expand-transition>
                <div v-if="!settings.useSunSchedule" class="time-picker-grid">
                  <v-card class="time-picker-card" flat border>
                    <div class="time-picker-header">
                      <v-icon color="warning">mdi-white-balance-sunny</v-icon>
                      <span class="time-picker-label">Turn on light theme</span>
                    </div>
                    <div class="time-picker-body">
                      <v-time-picker
                        rounded="lg"
                        bg-color="surface-container"
                        v-model="settings.enableLightThemeTime"
                        format="24hr"
                        scrollable
                        elevation="0"
                      />
                    </div>
                  </v-card>

                  <v-card class="time-picker-card" flat border>
                    <div class="time-picker-header">
                      <v-icon color="primary">mdi-weather-night</v-icon>
                      <span class="time-picker-label">Turn on dark theme</span>
                    </div>
                    <div class="time-picker-body">
                      <v-time-picker
                        rounded="lg"
                        bg-color="surface-container"
                        v-model="settings.enableDarkThemeTime"
                        format="24hr"
                        scrollable
                        elevation="0"
                      />
                    </div>
                  </v-card>
                </div>
              </v-expand-transition>
            </div>
          </v-expand-transition>

          <!-- Section: Color Settings -->
          <div class="section-divider color-section-divider">
            <span class="section-label">Color</span>
            <v-divider class="divider-line" />
          </div>

          <div class="color-settings-content">
            <v-switch
              color="primary"
              v-model="settings.useImageBackground"
              label="Use random photo as background"
              hide-details
              inset
              density="comfortable"
              class="background-switch"
            />

            <v-btn
              v-if="settings.useImageBackground"
              rounded
              variant="text"
              prepend-icon="mdi-shuffle-variant"
              @click="backgroundStore.newBackgroundTheme"
              color="primary"
              class="new-background-btn"
            >
              New background
            </v-btn>

            <v-slide-y-transition>
              <div v-if="!settings.useImageBackground" class="color-picker-wrapper">
                <span class="color-picker-title">Pick a Theme Seed Color</span>
                <v-card class="color-picker-card" flat border>
                  <v-color-picker
                    bg-color="transparent"
                    v-model="settings.customThemeColor"
                    hide-inputs
                    flat
                  />
                </v-card>
              </div>
            </v-slide-y-transition>
          </div>

          <!-- Section: Variant Options -->
          <div class="variant-settings">
            <span class="variant-title">Pick a Theme Variant</span>
            <v-chip-group v-model="settings.customThemeVariant" color="primary" mandatory column>
              <v-chip
                v-for="opt in themeVariantOptions"
                :value="opt"
                :key="opt"
                variant="flat"
                class="theme-chip"
              >
                {{ caps(opt) }}
              </v-chip>
            </v-chip-group>
          </div>
          <!-- Section: Contrast Settings -->
          <settings-slider
            v-model="settings.customThemeContrast"
            label="UI Contrast:"
            :min="-1.0"
            :max="1.0"
            :step="0.1"
            show-ticks
            :reset-value="CUSTOM_THEME_CONTRAST"
            :format-value="(val) => (val > 0 ? '+' : '') + val.toFixed(1)"
            description="Adjust color luminance contrast (with -1.0 as low-contrast, 0.0 as standard, and 1.0 as maximum contrast)."
          />
        </div>
      </v-card>
    </section>

    <!-- Active Theme Palette Visualizer -->
    <aside class="palette-visualizer">
      <v-card class="settings-card height-100" flat border>
        <div class="card-header">
          <span class="card-title">Active Swatches</span>
          <v-icon color="secondary" size="large">mdi-eyedropper</v-icon>
        </div>

        <div class="card-body">
          <p class="visualizer-desc">
            This visualization shows how your currently active scheme translates to different UI
            elements within the app.
          </p>

          <div class="swatch-grid">
            <v-card
              v-for="swatch in previewSwatches"
              :key="swatch.name"
              :class="[swatch.bg, swatch.text, 'swatch-card']"
              flat
            >
              <div class="swatch-content">
                <div class="swatch-header">
                  <div class="swatch-name">{{ swatch.name }}</div>
                  <div class="swatch-class-label">{{ swatch.bg }}</div>
                </div>
                <div class="swatch-desc-container">
                  {{ swatch.desc }}
                </div>
              </div>
            </v-card>
          </div>
        </div>
      </v-card>
    </aside>
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
