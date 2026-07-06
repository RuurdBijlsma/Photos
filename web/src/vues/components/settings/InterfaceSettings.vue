<script setup lang="ts">
import { TIMELINE_ROW_HEIGHT, useSettingStore } from '@/scripts/stores/settingsStore.ts'
import ViewPhoto from '@/vues/views/main/ViewPhoto.vue'
import { ref } from 'vue'
import { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import searchService from '@/scripts/services/searchService.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import { useEventListener } from '@vueuse/core'
import SettingsSlider from '@/vues/components/settings/components/SettingsSlider.vue'

const settings = useSettingStore()

const rowHeightEditing = ref(false)
useEventListener(document, 'mouseup', () => (rowHeightEditing.value = false))

const previewTimeline = ref<SimpleTimelineItem[]>([])
searchService.search({ query: 'sunset', limit: 10, mediaType: 'photo' }).then((items) => {
  previewTimeline.value = items.items
})
</script>

<template>
  <div class="ui-settings-layout">
    <simple-timeline
      v-if="rowHeightEditing && previewTimeline.length > 0"
      hide-scroll-bar
      class="timeline-preview"
      :timeline-items="previewTimeline"
      view-link="/unreachable"
      :ideal-row-height="settings.timelineRowHeight"
      :style="{
        height: settings.timelineRowHeight + 100 + 'px',
      }"
    />
    <!-- Settings Configuration Panel -->
    <section class="config-panel">
      <v-card class="settings-card" flat border>
        <!-- Card Header -->
        <div class="card-header">
          <span class="card-title">User Interface Options</span>
          <v-icon color="primary" size="large">mdi-monitor-dashboard</v-icon>
        </div>

        <div class="card-body">
          <!-- Section: Photo Viewer Settings -->
          <div class="section-divider">
            <span class="section-label">Photo Viewer</span>
            <v-divider class="divider-line" />
          </div>

          <div class="settings-group">
            <v-switch
              v-model="settings.darkPhotoViewer"
              label="Force Dark Mode in Viewer"
              hint="Always displays the photo viewer with a dark theme, regardless of app theme"
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch"
            />
            <v-switch
              v-model="settings.lightPhotoViewerMap"
              label="Force Light Mode in Viewer Map"
              hint="Always displays the map in the media info panel with a light theme, regardless of app theme"
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch"
            />
            <v-switch
              v-model="settings.useImageGlow"
              label="Ambient Image Glow"
              hint="Adds a soft glowing background reflecting the colors of the active image"
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch"
            />
            <v-switch
              v-model="settings.playMotionPhotos"
              label="Enable Motion Photos"
              hint="Automatically play motion photos when viewing items (if available)"
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch"
            />
          </div>

          <!-- Section: Timeline Settings -->
          <div class="section-divider mt-6">
            <span class="section-label">Timeline Layout</span>
            <v-divider class="divider-line" />
          </div>

          <div class="settings-group">
            <settings-slider
              v-model="settings.timelineRowHeight"
              label="Timeline Row Height:"
              unit="px"
              :min="50"
              :max="1000"
              :step="5"
              :reset-value="TIMELINE_ROW_HEIGHT"
              @slide-start="rowHeightEditing = true"
              description="<em>Target</em> height for the rows on the front page photo grid."
            />

            <v-switch
              v-model="settings.timelineUseDayLabels"
              label="Show Daily Group Headers"
              hint="Display date and location banners separating images by day"
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch mt-4"
            />
          </div>

          <!-- Section: General Aesthetics -->
          <div class="section-divider mt-6">
            <span class="section-label">Performance</span>
            <v-divider class="divider-line" />
          </div>

          <div class="settings-group">
            <v-switch
              v-model="settings.useBackdropBlur"
              label="Backdrop Blur Effects"
              hint="Enables blur on sidebars, dialogs, and headers. Disabling can improve UI performance."
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch"
            />
            <v-switch
              v-model="settings.asyncImageDecoding"
              label="Asynchronous image decoding"
              hint="Images are decoded in the background to reduce stuttering while browsing. This will make photos appear slower, but improves performance while scrolling through the timeline."
              persistent-hint
              color="primary"
              inset
              density="comfortable"
              class="setting-switch"
            />
          </div>
        </div>
      </v-card>
    </section>

    <!-- Interactive Live UI Preview -->
    <aside class="preview-panel">
      <v-card class="settings-card height-100" flat border>
        <div class="card-header">
          <span class="card-title">Live Preview</span>
          <v-icon color="secondary" size="large">mdi-eye-outline</v-icon>
        </div>

        <div class="card-body preview-container">
          <p class="preview-desc">
            Preview how your settings change the look and behavior of the application.
          </p>

          <!-- Preview Element 1: Photo Viewer & Glow -->
          <div class="preview-element mt-4" v-if="previewTimeline.length > 0">
            <div class="preview-element-title">Viewer Ambient Glow & Theme</div>
            <div class="viewer-preview-box">
              <div class="preview-scale">
                <view-photo disable-event-capture :override-id="previewTimeline[0]!.id" muted />
              </div>
            </div>
          </div>

          <!-- Preview Element 2: Backdrop blur -->
          <div class="preview-element mt-6">
            <div class="preview-element-title">Backdrop Blur</div>
            <div class="blur-preview-box">
              <!-- Background layer simulating actual UI behind -->
              <div class="blur-preview-bg-ui">
                <div class="mock-grid">
                  <div class="mock-grid-item" v-for="n in 3" :key="n">
                    <thumbnail-img
                      v-if="previewTimeline[n - 1]"
                      :media-item-id="previewTimeline[n - 1].id"
                      :height="144"
                      cover
                    />
                    <div v-else class="fallback-color-dot" :class="'dot-' + n"></div>
                  </div>
                </div>
                <div class="mock-text-lines">
                  <div class="mock-line title-line"></div>
                  <div class="mock-line text-line"></div>
                  <div class="mock-line text-line short"></div>
                </div>
              </div>

              <!-- Foreground overlay representing a dialog card -->
              <div class="mock-dialog-overlay">
                <div class="mock-dialog-card" :class="{ 'apply-blur': settings.useBackdropBlur }">
                  <div class="mock-dialog-header">
                    <v-icon size="small" class="mr-1">mdi-blur</v-icon>
                    <span>System Settings</span>
                  </div>
                  <div class="mock-dialog-body">
                    Backdrop blur status is currently shown as
                    <strong>{{ settings.useBackdropBlur ? 'enabled' : 'disabled' }}</strong
                    >.
                  </div>
                  <div class="mock-dialog-footer">
                    <div class="mock-dialog-btn">Settings</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </v-card>
    </aside>
  </div>
</template>

<style scoped>
.timeline-preview {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  opacity: 0.6;
  pointer-events: none;
}

.ui-settings-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 24px;
}

@media (min-width: 1280px) {
  .ui-settings-layout {
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

.section-label {
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-primary));
  white-space: nowrap;
}

.divider-line {
  margin-left: 16px;
  opacity: 0.3;
}

.settings-group {
  display: flex;
  flex-direction: column;
}

.preview-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 16px;
}

.preview-element {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.preview-element-title {
  font-size: 0.75rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-secondary));
}

.viewer-preview-box {
  border-radius: 12px;
  padding: 0;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  transition:
    background-color 0.3s ease,
    color 0.3s ease;
  background-color: rgb(var(--v-theme-surface-container-high));
  height: 133px;
  width: 100%;
  overflow: hidden;
  position: relative;
  margin: 0 auto;
}

.preview-scale {
  position: absolute;
  top: 0;
  left: 0;
  width: 300%;
  height: 400px;
  transform: scale(0.33333);
  transform-origin: top left;
  pointer-events: none;
}

/* Backdrop Blur Simulation */
.blur-preview-box {
  position: relative;
  height: 180px;
  border-radius: 16px;
  overflow: hidden;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  background-color: rgb(var(--v-theme-surface-container-low));
}

.blur-preview-bg-ui {
  position: absolute;
  inset: 0;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  z-index: 1;
}

.mock-grid {
  display: flex;
  gap: 8px;
}

.mock-grid-item {
  flex: 1;
  height: 80px;
  border-radius: 8px;
  overflow: hidden;
  background-color: rgb(var(--v-theme-surface-container-highest));
  position: relative;
  border: 1px solid rgba(var(--v-border-color), 0.15);
}

.fallback-color-dot {
  position: absolute;
  inset: 10px;
  border-radius: 50%;
}

.fallback-color-dot.dot-1 {
  background-color: rgb(var(--v-theme-primary));
}
.fallback-color-dot.dot-2 {
  background-color: rgb(var(--v-theme-secondary));
}
.fallback-color-dot.dot-3 {
  background-color: rgb(var(--v-theme-tertiary));
}

.mock-text-lines {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.mock-line {
  height: 8px;
  border-radius: 4px;
  background-color: rgb(var(--v-theme-on-surface));
  opacity: 0.15;
}

.mock-line.title-line {
  width: 40%;
  height: 12px;
  opacity: 0.25;
}

.mock-line.text-line {
  width: 85%;
}

.mock-line.text-line.short {
  width: 60%;
}

/* Foreground Modal Overlay with SearchBar Dropdown Styling */
.mock-dialog-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2;
  background-color: rgba(0, 0, 0, 0.15);
}

.mock-dialog-card {
  color: rgb(var(--v-theme-on-surface-container-high));
  border-radius: 16px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background-color: rgba(var(--v-theme-surface-container-high), 0.95);
  box-shadow: 0 10px 20px 0 rgba(0, 0, 0, 0.2);
  width: 80%;
  max-width: 280px;
  padding: 12px 16px;
  transition:
    backdrop-filter 0.3s ease,
    background-color 0.3s ease;
}

.mock-dialog-card.apply-blur {
  background-color: rgba(var(--v-theme-surface-container-high), 0.8);
  backdrop-filter: saturate(250%) blur(12px);
}

.mock-dialog-header {
  font-size: 0.85rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  margin-bottom: 6px;
  color: rgb(var(--v-theme-primary));
}

.mock-dialog-body {
  font-size: 0.75rem;
  line-height: 1.35;
  opacity: 0.85;
  margin-bottom: 8px;
}

.mock-dialog-body strong {
  color: rgb(var(--v-theme-primary));
}

.mock-dialog-footer {
  display: flex;
  justify-content: flex-end;
}

.mock-dialog-btn {
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-primary));
  padding: 4px 10px;
  border-radius: 6px;
  background-color: rgba(var(--v-theme-primary), 0.1);
  cursor: pointer;
}
</style>
