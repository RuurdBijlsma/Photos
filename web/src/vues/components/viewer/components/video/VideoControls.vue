<script setup lang="ts">
import MdiFullscreen from '~icons/mdi/fullscreen'
import MdiFullscreenExit from '~icons/mdi/fullscreen-exit'
import MdiPause from '~icons/mdi/pause'
import MdiPlay from '~icons/mdi/play'
import MdiVolumeHigh from '~icons/mdi/volume-high'
import MdiVolumeMedium from '~icons/mdi/volume-medium'
import MdiVolumeMute from '~icons/mdi/volume-mute'
import { computed } from 'vue'
import { toHms } from '@/scripts/utils'
import VideoProgressSlider from '@/vues/components/viewer/components/video/VideoProgressSlider.vue'
import VideoSettingsMenu from '@/vues/components/viewer/components/video/VideoSettingsMenu.vue'

const props = defineProps<{
  currentTime: number
  duration: number
  buffered: Array<{ start: number; end: number }>
  isPlaying: boolean
  isMuted: boolean
  volume: number
  showUi: boolean
  isFullscreen: boolean
  quality: number | 'source'
  sortedVideoSizes: number[]
  sourceHeight: number
  isSourceAvailable: boolean
  hasThumbnails: boolean
  currentPlaybackRate: number
  playbackRates: number[]
  attachFullscreen?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle-play'): void
  (e: 'toggle-mute'): void
  (e: 'toggle-fullscreen'): void
  (e: 'seek', time: number): void
  (e: 'update:volume', vol: number): void
  (e: 'update:quality', quality: number | 'source'): void
  (e: 'update:playbackRate', rate: number): void
}>()

const volumeIcon = computed(() => {
  if (props.isMuted || props.volume === 0) return MdiVolumeMute
  if (props.volume < 0.5) return MdiVolumeMedium
  return MdiVolumeHigh
})
</script>

<template>
  <div class="video-controls-container" :class="{ 'hide-ui': !showUi }">
    <!-- Progress Bar -->
    <div class="seekbar-row">
      <VideoProgressSlider
        :model-value="currentTime"
        :max="duration"
        :buffered="buffered"
        @update:model-value="emit('seek', $event)"
      />
    </div>

    <!-- Floating Capsules Row -->
    <div class="controls-row">
      <!-- Left Island -->
      <div class="control-island left-island">
        <v-btn
          variant="plain"
          :icon="isPlaying ? MdiPause : MdiPlay"
          rounded="xl"
          @click="emit('toggle-play')"
        />

        <div class="volume-container">
          <v-btn variant="plain" :icon="volumeIcon" rounded="xl" @click="emit('toggle-mute')" />
          <div class="volume-slider-wrapper">
            <v-slider
              class="volume-slider"
              :model-value="isMuted ? 0 : volume"
              @update:model-value="emit('update:volume', $event)"
              min="0"
              max="1"
              step="0.05"
              thumb-size="15"
              track-size="3"
              hide-details
              density="compact"
            />
          </div>
        </div>

        <div class="time-display">{{ toHms(currentTime) }} / {{ toHms(duration) }}</div>
      </div>

      <!-- Right Island -->
      <div class="control-island right-island">
        <VideoSettingsMenu
          :quality="quality"
          :sorted-video-sizes="sortedVideoSizes"
          :source-height="sourceHeight"
          :is-source-available="isSourceAvailable"
          :has-thumbnails="hasThumbnails"
          :current-playback-rate="currentPlaybackRate"
          :playback-rates="playbackRates"
          :attach-fullscreen="attachFullscreen"
          @update:quality="emit('update:quality', $event)"
          @update:playback-rate="emit('update:playbackRate', $event)"
        />

        <v-btn
          variant="plain"
          :icon="isFullscreen ? MdiFullscreenExit : MdiFullscreen"
          rounded="xl"
          @click="emit('toggle-fullscreen')"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-controls-container {
  position: absolute;
  bottom: 10px;
  left: 0;
  width: 100%;
  display: flex;
  flex-direction: column;
  z-index: 1510;
  transition:
    transform 0.2s ease,
    opacity 0.2s ease;
}

.video-controls-container.hide-ui {
  transform: translateY(80px);
  opacity: 0;
  pointer-events: none;
}

.seekbar-row {
  width: 100%;
  padding: 0 10px;
}

.controls-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px;
}

.control-island {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 24px;
  border-radius: 30px;
  height: 52px;
  background-color: rgba(var(--bg), 0.8);
  box-shadow: 0 3px 12px rgba(var(--bg), 0.15);
  border: 1px solid transparent;
}

body.backdrop-blur .control-island {
  backdrop-filter: blur(30px) saturate(150%) brightness(90%) contrast(90%);
  background-color: rgba(var(--bg), 0.5);
  border: 1px solid rgba(var(--fg), 0.1);
}

body.backdrop-blur .control-island:hover {
  background-color: rgba(var(--bg), 0.7);
}

.volume-container {
  display: flex;
  align-items: center;
}

.volume-slider-wrapper {
  display: flex;
  align-items: center;
  width: 0;
  opacity: 0;
  overflow: hidden;
  transition:
    width 0.25s ease-in-out,
    opacity 0.2s ease-in-out,
    margin 0.25s ease-in-out;
  pointer-events: none;
  margin-left: 0;
}

.volume-container:hover .volume-slider-wrapper,
.volume-container:focus-within .volume-slider-wrapper {
  width: 96px;
  opacity: 1;
  pointer-events: auto;
  margin-left: 0;
  padding-left: 8px;
}

.volume-slider {
  width: 80px;
  flex: none;
}

.time-display {
  font-family: Jost, sans-serif;
  font-size: 13px;
  color: rgba(var(--fg), 0.8);
  user-select: none;
  font-weight: 400;
  margin-left: 10px;
  margin-right: 15px;
}

/* Volume Slider Color overrides */
:deep(.volume-slider .v-slider-thumb) {
  color: rgba(var(--fg), 0.9) !important;
}

:deep(.volume-slider .v-slider-track__fill) {
  background: rgba(var(--fg), 0.5) !important;
}

:deep(.volume-slider .v-slider-track__background) {
  background: rgba(var(--fg), 0.5) !important;
}

:deep(.volume-slider .v-slider-thumb__ripple) {
  color: rgba(var(--fg), 0.3) !important;
}

:deep(.volume-slider .v-slider-thumb--focused .v-slider-thumb__surface::before) {
  opacity: 0;
}
</style>
