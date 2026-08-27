<script setup lang="ts">
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiCogOutline from '~icons/mdi/cog-outline'
import MdiFullscreen from '~icons/mdi/fullscreen'
import MdiFullscreenExit from '~icons/mdi/fullscreen-exit'
import MdiPause from '~icons/mdi/pause'
import MdiPlay from '~icons/mdi/play'
import MdiVolumeHigh from '~icons/mdi/volume-high'
import MdiVolumeMedium from '~icons/mdi/volume-medium'
import MdiVolumeMute from '~icons/mdi/volume-mute'
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { useEventListener, useStorage } from '@vueuse/core'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { VIDEO_SIZES } from '@/scripts/constants.ts'
import { getVideoHeight, toHms, useObjStorage } from '@/scripts/utils.ts'
import VideoProgressSlider from '@/vues/components/viewer/components/VideoProgressSlider.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useDelayedBoolean } from '@/scripts/composables/useDelayedBoolean.ts'

const props = withDefaults(
  defineProps<{
    mediaItemId: string
    muted: boolean
    showUi?: boolean
    autoplay?: boolean
    elementalFullscreen: boolean
  }>(),
  {
    showUi: true,
    autoplay: true,
  },
)

const mediaItemStore = useMediaItemStore()
const authStore = useAuthStore()

// Video Element Reference
const videoEl = useTemplateRef('videoElement')
const videoContainerEl = useTemplateRef('videoViewer')
const fps = computed(() => fullImage.value?.media_features?.video_fps || 30)

// Playback States
const isPlaying = ref(false)
const isVideoLoaded = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const bufferedRanges = ref<Array<{ start: number; end: number }>>([])

// Track loading state for delayed blanking (swaps fast path without flicker, unloads on slow path)
const isPendingLoad = computed(() => !isVideoLoaded.value)
const showBlank = useDelayedBoolean(isPendingLoad, 100)

// Play/Pause Overlay States
const overlayAction = ref<'play' | 'pause' | null>(null)
const overlayTrigger = ref(0)
let overlayTimeout: ReturnType<typeof setTimeout> | null = null

// Volume State (Persisted with useStorage)
const savedVolume = useStorage<number>('video-player-volume', 1.0)
const isMuted = useStorage<boolean>('video-player-muted', false)

// Playback Speed State (Persisted with useStorage)
const savedPlaybackRate = useStorage<number>('video-player-playback-rate', 1.0)
const playbackRates = [3.0, 2.0, 1.5, 1.0, 0.5, 0.25]

const currentPlaybackRate = computed<number>({
  get() {
    return savedPlaybackRate.value
  },
  set(val: number) {
    savedPlaybackRate.value = val
    if (videoEl.value) {
      videoEl.value.playbackRate = val
    }
  },
})

// Settings Menu Open State
const settingsMenuOpen = ref(false)

// Fullscreen State
const isFullscreen = ref(false)

// Fetch metadata from the Pinia store if it is missing
const fullImage = computed(() => mediaItemStore.anyMediaItems.get(props.mediaItemId))
const hasThumbnails = computed(() => fullImage.value?.has_thumbnails ?? true)

const isSourceAvailable = computed(() => {
  if (!authStore.isAuthenticated) return false
  const mimeType = fullImage.value?.media_features?.mime_type
  return isVideoStreamable(mimeType)
})

const sourceHeight = computed(() => {
  if (!fullImage.value) return 0
  return Math.min(fullImage.value.width, fullImage.value.height)
})

// Quality State (Persisted with useStorage)
const defaultQuality = getVideoHeight(screen.height)
const savedQuality = useObjStorage<number | 'source'>('video-player-quality', defaultQuality)
const sortedVideoSizes = [...VIDEO_SIZES].sort((a, b) => b - a)

const currentQuality = computed<number | 'source'>({
  get() {
    const saved = savedQuality.value
    if (!fullImage.value) {
      // If metadata is not loaded yet, assume the saved quality is available
      return saved || defaultQuality
    }

    if (authStore.isAuthenticated) {
      if (!hasThumbnails.value) return 'source'
      if (saved === 'source') {
        if (isSourceAvailable.value) return 'source'
        return sortedVideoSizes[0]
      }
    }
    const numQuality = Number(saved)
    if (VIDEO_SIZES.includes(numQuality)) return numQuality
    return defaultQuality
  },
  set(val: number | 'source') {
    savedQuality.value = val
  },
})

// Computed video source URL based on quality and thumbnail generation availability
const videoUrl = computed(() => {
  if (currentQuality.value === 'source') {
    return mediaItemService.getVideo(props.mediaItemId, 0, true)
  }
  const onDemand = !hasThumbnails.value
  if (onDemand && !authStore.isAuthenticated) {
    return null
  }
  return mediaItemService.getVideo(props.mediaItemId, currentQuality.value as number, onDemand)
})

// Keep track of the position and play state to restore across source swaps
const timeToRestore = ref<number | null>(null)
const isPlayingOnQualityChange = ref<boolean | null>(null)

function onQualitySelect(size: number | 'source') {
  if (videoEl.value) {
    timeToRestore.value = videoEl.value.currentTime
    isPlayingOnQualityChange.value = !videoEl.value.paused
  }
  currentQuality.value = size
  settingsMenuOpen.value = false
}

function onPlaybackRateSelect(rate: number) {
  currentPlaybackRate.value = rate
  settingsMenuOpen.value = false
}

// Native browser-streamable support check
function isVideoStreamable(mimeType?: string): boolean {
  if (!mimeType) return false
  const lower = mimeType.toLowerCase()
  return (
    lower === 'video/mp4' ||
    lower === 'video/webm' ||
    lower === 'video/ogg' ||
    lower === 'video/quicktime'
  )
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
  if (videoEl.value) {
    const b = videoEl.value.buffered
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
  if (videoEl.value) {
    duration.value = videoEl.value.duration || 0
    if (timeToRestore.value !== null) {
      videoEl.value.currentTime = timeToRestore.value
      currentTime.value = timeToRestore.value
      timeToRestore.value = null
    }
    videoEl.value.playbackRate = currentPlaybackRate.value
    updateBufferedProgress()
  }
}

// Helper to trigger safe programmatic playback
function playVideo() {
  if (videoEl.value) {
    videoEl.value.play().catch((err) => {
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
      if (videoEl.value) {
        const shouldPlay =
          isPlayingOnQualityChange.value !== null ? isPlayingOnQualityChange.value : true

        videoEl.value.load()

        if (shouldPlay) {
          playVideo()
        } else {
          videoEl.value.pause()
        }
        videoEl.value.playbackRate = currentPlaybackRate.value

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
    if (videoEl.value) {
      videoEl.value.volume = savedVolume.value
      videoEl.value.muted = isMuted.value
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

// Reset loaded state on media item id changes
watch(
  () => props.mediaItemId,
  () => {
    isVideoLoaded.value = false
  },
)

// Animation loop to ensure smooth, high-precision progress updates during active playback
let animationFrameId: number | null = null

function updateProgressSmoothly() {
  if (videoEl.value && !videoEl.value.paused) {
    currentTime.value = videoEl.value.currentTime
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
  if (videoEl.value && videoEl.value.paused) {
    currentTime.value = videoEl.value.currentTime
  }
  updateBufferedProgress()
}

function onProgress() {
  updateBufferedProgress()
}

// Playback Controls
function togglePlay(showOverlay = false) {
  if (!videoEl.value) return
  if (videoEl.value.paused) {
    playVideo()
    if (showOverlay) triggerOverlay('play')
  } else {
    videoEl.value.pause()
    if (showOverlay) triggerOverlay('pause')
  }
}

// Seeking Control Helpers
function seekBy(seconds: number) {
  if (!videoEl.value) return
  let target = videoEl.value.currentTime + seconds
  if (target < 0) target = 0
  if (target > duration.value) target = duration.value
  videoEl.value.currentTime = target
  currentTime.value = target
}

function stepFrame(direction: number) {
  if (!videoEl.value) return
  const frameDuration = 1 / fps.value
  let target = videoEl.value.currentTime + direction * frameDuration
  if (target < 0) target = 0
  if (target > duration.value) target = duration.value
  videoEl.value.currentTime = target
  currentTime.value = target
}

function onSeekInput(val: number) {
  if (videoEl.value) {
    videoEl.value.currentTime = val
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
    if (props.elementalFullscreen && videoContainerEl.value) {
      videoContainerEl.value.requestFullscreen().catch((err) => {
        console.error('Failed to enter fullscreen mode:', err)
      })
    } else {
      document.documentElement.requestFullscreen().catch((err) => {
        console.error('Failed to enter fullscreen mode:', err)
      })
    }
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
      if (videoEl.value?.paused) {
        e.preventDefault()
        stepFrame(-1)
      }
      break
    case '.':
      if (videoEl.value?.paused) {
        e.preventDefault()
        stepFrame(1)
      }
      break
    case 'f11':
      e.preventDefault()
      toggleFullscreen()
      break
  }
}

const volumeIcon = computed(() => {
  if (isMuted.value || savedVolume.value === 0) return MdiVolumeMute
  if (savedVolume.value < 0.5) return MdiVolumeMedium
  return MdiVolumeHigh
})

onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
  if (videoEl.value && isPlaying.value === false && props.autoplay) {
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
  <div class="video-viewer" ref="videoViewer">
    <video
      v-if="videoUrl"
      ref="videoElement"
      class="video-element"
      :src="videoUrl"
      :muted="isMuted"
      :crossorigin="currentQuality === 'source' ? 'use-credentials' : undefined"
      :style="{
        opacity: isVideoLoaded || !showBlank ? 1 : 0,
      }"
      loop
      playsinline
      @loadeddata="isVideoLoaded = true"
      @loadedmetadata="onLoadedMetadata"
      @timeupdate="onTimeUpdate"
      @progress="onProgress"
      @play="onPlay"
      @pause="onPause"
      @click="togglePlay(true)"
      @dblclick="toggleFullscreen"
    />
    <div v-else class="still-processing">
      <span>Video is still processing, check back later to watch it</span>
    </div>

    <!-- Play/Pause Overlay Indication -->
    <div v-if="overlayAction" :key="overlayTrigger" class="play-pause-overlay">
      <div class="overlay-circle">
        <v-icon :icon="overlayAction === 'play' ? MdiPlay : MdiPause" size="40" />
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
            :icon="isPlaying ? MdiPause : MdiPlay"
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
                thumb-size="15"
                track-size="3"
                hide-details
                density="compact"
              />
            </div>
          </div>

          <div class="time-display">{{ toHms(currentTime) }} / {{ toHms(duration) }}</div>
        </div>

        <!-- Right Island Capsule -->
        <div class="control-island right-island">
          <!-- Quality & Speed Settings Menu -->
          <v-menu
            :attach="elementalFullscreen && isFullscreen"
            v-if="hasThumbnails || isSourceAvailable"
            v-model="settingsMenuOpen"
            location="top center"
            :close-on-content-click="false"
          >
            <template v-slot:activator="{ props }">
              <v-btn variant="plain" :icon="MdiCogOutline" rounded="xl" v-bind="props" />
            </template>
            <v-list class="settings-menu-list">
              <!-- Submenu 1: Playback Speed -->
              <v-menu
                location="left top"
                open-on-hover
                :close-on-content-click="true"
                :attach="elementalFullscreen && isFullscreen"
              >
                <template v-slot:activator="{ props: speedMenuProps }">
                  <v-list-item v-bind="speedMenuProps" class="menu-item-with-chevron">
                    <v-list-item-title class="menu-text">Playback speed</v-list-item-title>
                    <template v-slot:append>
                      <span class="current-setting-label">{{ currentPlaybackRate }}x</span>
                      <v-icon :icon="MdiChevronRight" size="small" class="ml-1" />
                    </template>
                  </v-list-item>
                </template>
                <v-list class="submenu-list">
                  <v-list-item
                    v-for="rate in playbackRates"
                    :key="rate"
                    :value="rate"
                    @click="onPlaybackRateSelect(rate)"
                    :active="currentPlaybackRate === rate"
                  >
                    <v-list-item-title class="menu-text">{{ rate }}x</v-list-item-title>
                  </v-list-item>
                </v-list>
              </v-menu>

              <!-- Submenu 2: Quality -->
              <v-menu
                location="left top"
                open-on-hover
                :close-on-content-click="true"
                :attach="elementalFullscreen && isFullscreen"
              >
                <template v-slot:activator="{ props: qualityMenuProps }">
                  <v-list-item v-bind="qualityMenuProps" class="menu-item-with-chevron">
                    <v-list-item-title class="menu-text">Quality</v-list-item-title>
                    <template v-slot:append>
                      <span class="current-setting-label">
                        {{
                          currentQuality === 'source' ? `${sourceHeight}p` : `${currentQuality}p`
                        }}
                      </span>
                      <v-icon :icon="MdiChevronRight" size="small" class="ml-1" />
                    </template>
                  </v-list-item>
                </template>
                <v-list class="submenu-list">
                  <v-list-item
                    v-if="isSourceAvailable"
                    value="source"
                    @click="onQualitySelect('source')"
                    :active="currentQuality === 'source'"
                  >
                    <v-list-item-title class="menu-text">
                      {{ sourceHeight }}p <span class="source-label">(source)</span>
                    </v-list-item-title>
                  </v-list-item>

                  <template v-if="hasThumbnails">
                    <v-list-item
                      v-for="size in sortedVideoSizes"
                      :key="size"
                      :value="size"
                      @click="onQualitySelect(size)"
                      :active="currentQuality === size"
                    >
                      <v-list-item-title class="menu-text">{{ size }}p</v-list-item-title>
                    </v-list-item>
                  </template>
                </v-list>
              </v-menu>
            </v-list>
          </v-menu>
          <v-btn v-else variant="plain" :icon="MdiCogOutline" rounded="xl" disabled />

          <!-- Fullscreen Button -->
          <v-btn
            variant="plain"
            :icon="isFullscreen ? MdiFullscreenExit : MdiFullscreen"
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

.still-processing {
  width: 100%;
  height: 100%;
  display: flex;
  place-items: center;
  place-content: center;
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

.settings-menu-list {
  min-width: 220px;
}

.submenu-list {
  min-width: 140px;
}

.menu-text {
  font-family: Jost, sans-serif;
  font-weight: 500;
}

.current-setting-label {
  font-family: Jost, sans-serif;
  font-size: 13px;
  opacity: 0.6;
}

.source-label {
  opacity: 0.6;
  font-size: 0.85em;
  font-weight: 400;
  margin-left: 4px;
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

:deep(.volume-slider .v-slider-thumb--focused .v-slider-thumb__surface::before) {
  opacity: 0;
}
</style>
