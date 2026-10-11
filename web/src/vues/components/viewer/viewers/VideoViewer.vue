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
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { useEventListener, useStorage } from '@vueuse/core'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { VIDEO_SIZES } from '@/scripts/constants.ts'
import { getVideoHeight, toHms, useObjStorage } from '@/scripts/utils.ts'
import VideoProgressSlider from '@/vues/components/viewer/components/VideoProgressSlider.vue'
import VideoPlayer from '@/vues/components/viewer/viewers/VideoPlayer.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

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

const emit = defineEmits<{
  (e: 'buffering', isBuffering: boolean): void
}>()

const mediaItemStore = useMediaItemStore()
const authStore = useAuthStore()

// DOM & Component References
const videoPlayerRef = useTemplateRef<InstanceType<typeof VideoPlayer>>('videoPlayer')
const videoContainerEl = useTemplateRef<HTMLDivElement>('videoViewer')

// Metadata & Capabilities
const fullImage = computed(() => mediaItemStore.anyMediaItems.get(props.mediaItemId))
const fps = computed(() => fullImage.value?.media_features?.video_fps || 30)
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

// Playback States
const isPlaying = ref(false)
const isVideoLoaded = ref(false)
const isBuffering = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const bufferedRanges = ref<Array<{ start: number; end: number }>>([])

// Play/Pause Center Indicator Overlay
const overlayAction = ref<'play' | 'pause' | null>(null)
const overlayTrigger = ref(0)
let overlayTimeout: ReturnType<typeof setTimeout> | null = null

// Volume State (Persisted)
const savedVolume = useStorage<number>('video-player-volume', 1.0)
const isMuted = useStorage<boolean>('video-player-muted', false)

// Playback Speed State (Persisted)
const savedPlaybackRate = useStorage<number>('video-player-playback-rate', 1.0)
const playbackRates = [3.0, 2.0, 1.5, 1.0, 0.5, 0.25]

const currentPlaybackRate = computed<number>({
  get() {
    return savedPlaybackRate.value
  },
  set(val: number) {
    savedPlaybackRate.value = val
  },
})

// Settings Menu Open State
const settingsMenuOpen = ref(false)

// Fullscreen State
const isFullscreen = ref(false)

// Quality State (Persisted)
const defaultQuality = getVideoHeight(screen.height)
const savedQuality = useObjStorage<number | 'source'>('video-player-quality', defaultQuality)
const sortedVideoSizes = [...VIDEO_SIZES].sort((a, b) => b - a)

const currentQuality = computed<number | 'source'>({
  get() {
    const saved = savedQuality.value
    if (!fullImage.value) {
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

// Video Source URL resolution
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

// Forward buffering flag to parent
const isBufferingOrLoading = computed(() => {
  if (!videoUrl.value) return false
  return !isVideoLoaded.value || isBuffering.value
})

watch(
  isBufferingOrLoading,
  (val) => {
    emit('buffering', val)
  },
  { immediate: true },
)

function onQualitySelect(size: number | 'source') {
  currentQuality.value = size
  settingsMenuOpen.value = false
}

function onPlaybackRateSelect(rate: number) {
  currentPlaybackRate.value = rate
  settingsMenuOpen.value = false
}

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

function triggerOverlay(action: 'play' | 'pause') {
  overlayAction.value = action
  overlayTrigger.value++
  if (overlayTimeout !== null) clearTimeout(overlayTimeout)
  overlayTimeout = window.setTimeout(() => {
    overlayAction.value = null
    overlayTimeout = null
  }, 750)
}

function togglePlay(showOverlay = false) {
  if (!videoPlayerRef.value) return
  const willPlay = !isPlaying.value
  videoPlayerRef.value.togglePlay()
  if (showOverlay) {
    triggerOverlay(willPlay ? 'play' : 'pause')
  }
}

function seekBy(seconds: number) {
  videoPlayerRef.value?.seekBy(seconds)
}

function stepFrame(direction: number) {
  videoPlayerRef.value?.stepFrame(direction)
}

function onSeekInput(val: number) {
  videoPlayerRef.value?.seek(val)
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

function adjustVolume(amount: number) {
  let target = savedVolume.value + amount
  if (target < 0) target = 0
  if (target > 1) target = 1
  savedVolume.value = parseFloat(target.toFixed(2))
  if (target > 0) {
    isMuted.value = false
  }
}

// Fullscreen API handling
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

// Global Keyboard shortcuts
function handleKeyDown(e: KeyboardEvent) {
  const target = e.target as HTMLElement
  if (target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA' || target?.isContentEditable) {
    return
  }

  switch (e.key.toLowerCase()) {
    case ' ':
      e.preventDefault()
      togglePlay(true)
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
      if (!isPlaying.value) {
        e.preventDefault()
        stepFrame(-1)
      }
      break
    case '.':
      if (!isPlaying.value) {
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

// Mute synchronization with parent prop
watch(
  () => props.muted,
  () => {
    if (props.muted) {
      isMuted.value = props.muted
    }
  },
  { immediate: true },
)

onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
})

onUnmounted(() => {
  emit('buffering', false)
  document.removeEventListener('fullscreenchange', onFullscreenChange)
})

useEventListener(window, 'keydown', handleKeyDown)
</script>

<template>
  <div class="video-viewer" ref="videoViewer">
    <VideoPlayer
      v-if="videoUrl"
      ref="videoPlayer"
      :src="videoUrl"
      :media-item-id="mediaItemId"
      :muted="isMuted"
      :volume="savedVolume"
      :playback-rate="currentPlaybackRate"
      :crossorigin="currentQuality === 'source' ? 'use-credentials' : undefined"
      :autoplay="autoplay"
      :fps="fps"
      @update:current-time="currentTime = $event"
      @update:duration="duration = $event"
      @update:is-playing="isPlaying = $event"
      @update:is-buffering="isBuffering = $event"
      @update:is-video-loaded="isVideoLoaded = $event"
      @update:buffered-ranges="bufferedRanges = $event"
      @click-video="togglePlay(true)"
      @dblclick-video="toggleFullscreen"
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
      <!-- Progress Slider -->
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
          <v-btn
            variant="plain"
            :icon="isPlaying ? MdiPause : MdiPlay"
            rounded="xl"
            @click="togglePlay()"
          />

          <!-- Hover-revealed volume controls -->
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
            <template v-slot:activator="{ props: menuActivatorProps }">
              <v-btn
                variant="plain"
                :icon="MdiCogOutline"
                rounded="xl"
                v-bind="menuActivatorProps"
              />
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
