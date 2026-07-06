<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useStorage, useEventListener } from '@vueuse/core'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { VIDEO_SIZES } from '@/scripts/constants.ts'
import { getVideoHeight, toHms } from '@/scripts/utils.ts'
import VideoProgressSlider from '@/vues/components/viewer/components/VideoProgressSlider.vue'

const props = withDefaults(
  defineProps<{
    mediaItemId: string
    muted: boolean
    showUi?: boolean
  }>(),
  {
    showUi: true,
  },
)

const mediaItemStore = useMediaItemStore()

// Video Element Reference
const videoRef = ref<HTMLVideoElement | null>(null)
const fps = computed(() => fullImage.value?.media_features?.video_fps || 30)

// Playback States
const isPlaying = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const bufferedRanges = ref<Array<{ start: number; end: number }>>([])

// Play/Pause Overlay States
const overlayAction = ref<'play' | 'pause' | null>(null)
const overlayTrigger = ref(0)
let overlayTimeout: ReturnType<typeof setTimeout> | null = null

// Volume State (Persisted with useStorage)
const savedVolume = useStorage<number>('video-player-volume', 1.0)
const isMuted = useStorage<boolean>('video-player-muted', false)

// Fullscreen State
const isFullscreen = ref(false)

// Quality State (Persisted with useStorage)
const defaultQuality = getVideoHeight(screen.height)
const savedQuality = useStorage<number>('video-player-quality', defaultQuality)
const sortedVideoSizes = [...VIDEO_SIZES].sort((a, b) => b - a)

const currentQuality = computed({
  get() {
    if (VIDEO_SIZES.includes(savedQuality.value)) {
      return savedQuality.value
    }
    return defaultQuality
  },
  set(val: number) {
    savedQuality.value = val
  },
})

// Fetch metadata from the Pinia store if it is missing
const fullImage = computed(() => mediaItemStore.mediaItems.get(props.mediaItemId))
const hasThumbnails = computed(() => fullImage.value?.has_thumbnails ?? true)

// Computed video source URL based on quality and thumbnail generation availability
const videoUrl = computed(() => {
  const onDemand = !hasThumbnails.value
  return mediaItemService.getVideo(props.mediaItemId, currentQuality.value, onDemand)
})

// Keep track of the position and play state to restore across source swaps
const timeToRestore = ref<number | null>(null)
const isPlayingOnQualityChange = ref<boolean | null>(null)

function onQualitySelect(size: number) {
  if (videoRef.value) {
    timeToRestore.value = videoRef.value.currentTime
    isPlayingOnQualityChange.value = !videoRef.value.paused
  }
  currentQuality.value = size
}

// Triggers the temporary play/pause animation overlay in the middle of the screen
function triggerOverlay(action: 'play' | 'pause') {
  overlayAction.value = action
  overlayTrigger.value++
  if (overlayTimeout !== null) clearTimeout(overlayTimeout)
  overlayTimeout = window.setTimeout(() => {
    overlayAction.value = null
    overlayTimeout = null
  }, 750)
}

// Queries current media buffering intervals directly from the browser instance
function updateBufferedProgress() {
  if (videoRef.value) {
    const b = videoRef.value.buffered
    const ranges: Array<{ start: number; end: number }> = []
    for (let i = 0; i < b.length; i++) {
      ranges.push({
        start: b.start(i),
        end: b.end(i),
      })
    }
    bufferedRanges.value = ranges
  }
}

function onLoadedMetadata() {
  if (videoRef.value) {
    duration.value = videoRef.value.duration || 0
    if (timeToRestore.value !== null) {
      videoRef.value.currentTime = timeToRestore.value
      currentTime.value = timeToRestore.value
      timeToRestore.value = null
    }
    updateBufferedProgress()
  }
}

// Watch for mediaItemId changes and fetch if not present in the current view context
watch(
  () => props.mediaItemId,
  async (newId) => {
    if (newId && !mediaItemStore.mediaItems.has(newId)) {
      try {
        await mediaItemStore.fetchMediaItem(newId)
      } catch (e) {
        console.error('Failed to fetch media item for video viewer:', e)
      }
    }
  },
  { immediate: true },
)

// Helper to trigger safe programmatic playback
function playVideo() {
  if (videoRef.value) {
    videoRef.value.play().catch((err) => {
      console.warn('Playback failed or was blocked by browser:', err)
    })
  }
}

// Trigger reload on source changes
watch(
  videoUrl,
  () => {
    bufferedRanges.value = [] // Reset buffering indicator layout
    nextTick(() => {
      if (videoRef.value) {
        const shouldPlay =
          isPlayingOnQualityChange.value !== null ? isPlayingOnQualityChange.value : true

        videoRef.value.load()

        if (shouldPlay) {
          playVideo()
        } else {
          videoRef.value.pause()
        }

        isPlayingOnQualityChange.value = null
      }
    })
  },
  { immediate: true },
)

// Volume & Mute Synchronization
watch(
  [savedVolume, isMuted],
  () => {
    if (videoRef.value) {
      videoRef.value.volume = savedVolume.value
      videoRef.value.muted = isMuted.value
    }
  },
  { immediate: true },
)
watch(
  () => props.muted,
  () => {
    if (props.muted) {
      isMuted.value = props.muted
    }
  },
  { immediate: true },
)

// Animation loop to ensure smooth, high-precision progress updates during active playback
let animationFrameId: number | null = null

function updateProgressSmoothly() {
  if (videoRef.value && !videoRef.value.paused) {
    currentTime.value = videoRef.value.currentTime
    updateBufferedProgress()
    animationFrameId = requestAnimationFrame(updateProgressSmoothly)
  }
}

function onPlay() {
  isPlaying.value = true
  if (animationFrameId === null) {
    animationFrameId = requestAnimationFrame(updateProgressSmoothly)
  }
}

function onPause() {
  isPlaying.value = false
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }
}

// Fallback listener for captures while paused
function onTimeUpdate() {
  if (videoRef.value && videoRef.value.paused) {
    currentTime.value = videoRef.value.currentTime
  }
  updateBufferedProgress()
}

function onProgress() {
  updateBufferedProgress()
}

// Playback Controls
function togglePlay(showOverlay = false) {
  if (!videoRef.value) return
  if (videoRef.value.paused) {
    playVideo()
    if (showOverlay) triggerOverlay('play')
  } else {
    videoRef.value.pause()
    if (showOverlay) triggerOverlay('pause')
  }
}

// Seeking Control Helpers
function seekBy(seconds: number) {
  if (!videoRef.value) return
  let target = videoRef.value.currentTime + seconds
  if (target < 0) target = 0
  if (target > duration.value) target = duration.value
  videoRef.value.currentTime = target
  currentTime.value = target
}

function stepFrame(direction: number) {
  if (!videoRef.value) return
  const frameDuration = 1 / fps.value
  let target = videoRef.value.currentTime + direction * frameDuration
  if (target < 0) target = 0
  if (target > duration.value) target = duration.value
  videoRef.value.currentTime = target
  currentTime.value = target
}

function onSeekInput(val: number) {
  if (videoRef.value) {
    videoRef.value.currentTime = val
    currentTime.value = val
  }
}

function toggleMute() {
  isMuted.value = !isMuted.value
}

function onVolumeChange(val: number) {
  savedVolume.value = val
  if (val > 0) {
    isMuted.value = false
  }
}

// Volume Keyboard Helper
function adjustVolume(amount: number) {
  let target = savedVolume.value + amount
  if (target < 0) target = 0
  if (target > 1) target = 1
  savedVolume.value = parseFloat(target.toFixed(2))
  if (target > 0) {
    isMuted.value = false
  }
}

// Fullscreen API Handling (Syncs state on browser escape/system fullscreen change)
function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen().catch((err) => {
      console.error('Failed to enter fullscreen mode:', err)
    })
  } else {
    document.exitFullscreen()
  }
}

function onFullscreenChange() {
  isFullscreen.value = !!document.fullscreenElement
}

// Keyboard shortcuts logic
function handleKeyDown(e: KeyboardEvent) {
  const target = e.target as HTMLElement
  if (target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA' || target?.isContentEditable) {
    return
  }

  switch (e.key.toLowerCase()) {
    case ' ':
      e.preventDefault()
      togglePlay(true) // Triggers play/pause with screen animation feedback
      break
    case 'arrowleft':
      e.preventDefault()
      seekBy(-5)
      break
    case 'arrowright':
      e.preventDefault()
      seekBy(5)
      break
    case 'arrowup':
      e.preventDefault()
      adjustVolume(0.05)
      break
    case 'arrowdown':
      e.preventDefault()
      adjustVolume(-0.05)
      break
    case 'm':
      e.preventDefault()
      toggleMute()
      break
    case 'f':
      e.preventDefault()
      toggleFullscreen()
      break
    case ',':
      if (videoRef.value?.paused) {
        e.preventDefault()
        stepFrame(-1)
      }
      break
    case '.':
      if (videoRef.value?.paused) {
        e.preventDefault()
        stepFrame(1)
      }
      break
  }
}

const volumeIcon = computed(() => {
  if (isMuted.value || savedVolume.value === 0) return 'mdi-volume-mute'
  if (savedVolume.value < 0.5) return 'mdi-volume-medium'
  return 'mdi-volume-high'
})

onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
  if (videoRef.value && isPlaying.value === false) {
    playVideo()
  }
})

onUnmounted(() => {
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
  }
})

// Listen globally for media controls keyboard shortcuts
useEventListener(window, 'keydown', handleKeyDown)
</script>

<template>
  <div class="video-viewer">
    <!-- Video element (stretches to fill page while maintaining aspect ratio) -->
    <video
      ref="videoRef"
      class="video-element"
      :src="videoUrl"
      :muted="isMuted"
      loop
      playsinline
      @loadedmetadata="onLoadedMetadata"
      @timeupdate="onTimeUpdate"
      @progress="onProgress"
      @play="onPlay"
      @pause="onPause"
      @click="togglePlay(true)"
      @dblclick="toggleFullscreen"
    />

    <!-- Play/Pause Overlay Indication -->
    <div v-if="overlayAction" :key="overlayTrigger" class="play-pause-overlay">
      <div class="overlay-circle">
        <v-icon :icon="overlayAction === 'play' ? 'mdi-play' : 'mdi-pause'" size="40" />
      </div>
    </div>

    <!-- Custom Player Controls Bar -->
    <div class="video-controls-container" :class="{ 'hide-ui': !showUi }">
      <!-- Custom Progress Slider with hover tooltip and buffered segments -->
      <div class="seekbar-row">
        <VideoProgressSlider
          :model-value="currentTime"
          @update:model-value="onSeekInput"
          :max="duration"
          :buffered="bufferedRanges"
        />
      </div>

      <!-- Controls Floating Capsules Row -->
      <div class="controls-row">
        <!-- Left Island Capsule -->
        <div class="control-island left-island">
          <!-- Clicking bottom-left control capsule buttons will play/pause silently without screen overlay feedback -->
          <v-btn
            variant="plain"
            :icon="isPlaying ? 'mdi-pause' : 'mdi-play'"
            rounded="xl"
            @click="togglePlay()"
          />

          <!-- Hover-revealed volume controls container -->
          <div class="volume-container">
            <v-btn variant="plain" :icon="volumeIcon" rounded="xl" @click="toggleMute" />
            <div class="volume-slider-wrapper">
              <v-slider
                class="volume-slider"
                :model-value="isMuted ? 0 : savedVolume"
                @update:model-value="onVolumeChange"
                min="0"
                max="1"
                step="0.05"
                hide-details
                density="compact"
              />
            </div>
          </div>

          <div class="time-display">{{ toHms(currentTime) }} / {{ toHms(duration) }}</div>
        </div>

        <!-- Right Island Capsule -->
        <div class="control-island right-island">
          <!-- Quality Selector Menu -->
          <v-menu v-if="hasThumbnails" location="top center">
            <template v-slot:activator="{ props }">
              <v-btn variant="plain" icon="mdi-cog-outline" rounded="xl" v-bind="props" />
            </template>
            <v-list class="quality-menu-list">
              <v-list-item
                v-for="size in sortedVideoSizes"
                :key="size"
                :value="size"
                @click="onQualitySelect(size)"
                :active="currentQuality === size"
              >
                <v-list-item-title class="menu-text">{{ size }}p</v-list-item-title>
              </v-list-item>
            </v-list>
          </v-menu>
          <v-btn v-else variant="plain" icon="mdi-cog-outline" rounded="xl" disabled />

          <!-- Fullscreen Button -->
          <v-btn
            variant="plain"
            :icon="isFullscreen ? 'mdi-fullscreen-exit' : 'mdi-fullscreen'"
            rounded="xl"
            @click="toggleFullscreen"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.video-viewer {
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
  background-color: black;
  color: white;
  display: flex;
  place-items: center;
  justify-content: center;
  overflow: hidden;
  --bg: var(--v-theme-surface-container-lowest);
  --fg: var(--v-theme-on-surface-container-lowest);
}

.video-element {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

/* Play/Pause Center Indicator */
.play-pause-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 1505;
  pointer-events: none;
  animation: fade-out-overlay 750ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
}

.overlay-circle {
  width: 80px;
  height: 80px;
  background-color: rgba(0, 0, 0, 0.3);
  color: rgba(255, 255, 255, 0.8);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

@keyframes fade-out-overlay {
  0% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(0.85);
  }
  15% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1.1);
  }
  100% {
    opacity: 0;
    transform: translate(-50%, -50%) scale(1);
  }
}

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

/* Transitions the container width and opacity when hovering */
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
  width: 80px;
  opacity: 1;
  pointer-events: auto;
  margin-left: 8px;
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

.menu-text {
  font-family: Jost, sans-serif;
  font-weight: 500;
}

/* Custom Grayscale / Color-Neutral overrides for remaining Sliders (Decoupled) */

/* Volume Slider Customization */
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
</style>
