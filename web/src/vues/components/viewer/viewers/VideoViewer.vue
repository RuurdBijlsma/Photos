<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { useEventListener, useStorage } from '@vueuse/core'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { VIDEO_SIZES } from '@/scripts/constants.ts'
import { getVideoHeight, useObjStorage } from '@/scripts/utils.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useDelayedBoolean } from '@/scripts/composables/useDelayedBoolean.ts'
import VideoPlayPauseOverlay from '@/vues/components/viewer/components/video/VideoPlayPauseOverlay.vue'
import VideoControls from '@/vues/components/viewer/components/video/VideoControls.vue'

const props = withDefaults(
  defineProps<{
    mediaItemId: string
    muted: boolean
    showUi?: boolean
    autoplay?: boolean
    elementalFullscreen: boolean
    active?: boolean
  }>(),
  {
    showUi: true,
    autoplay: true,
    active: true,
  },
)

const emit = defineEmits<{
  (e: 'buffering', isBuffering: boolean): void
}>()

const mediaItemStore = useMediaItemStore()
const authStore = useAuthStore()

// Video Element Reference
const videoEl = useTemplateRef('videoElement')
const videoContainerEl = useTemplateRef('videoViewer')

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

// Track loading state for delayed blanking
const isPendingLoad = computed(() => !isVideoLoaded.value)
const showBlank = useDelayedBoolean(isPendingLoad, 100)

// Play/Pause Overlay States
const overlayAction = ref<'play' | 'pause' | null>(null)
const overlayTrigger = ref(0)
let overlayTimeout: ReturnType<typeof setTimeout> | null = null

// Volume State
const savedVolume = useStorage<number>('video-player-volume', 1.0)
const isMuted = useStorage<boolean>('video-player-muted', false)

// Playback Speed State
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

// Fullscreen State
const isFullscreen = ref(false)

// Quality State
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

// Video Source URL
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

const isBufferingOrLoading = computed(() => {
  if (!props.active || !videoUrl.value) return false
  return !isVideoLoaded.value || isBuffering.value
})

watch(
  [isBufferingOrLoading, () => props.active],
  ([val, active]) => {
    if (active) {
      emit('buffering', val)
    } else {
      emit('buffering', false)
    }
  },
  { immediate: true },
)

// Position and play state restoration across quality change
const timeToRestore = ref<number | null>(null)
const isPlayingOnQualityChange = ref<boolean | null>(null)

function onQualitySelect(size: number | 'source') {
  if (videoEl.value) {
    timeToRestore.value = videoEl.value.currentTime
    isPlayingOnQualityChange.value = !videoEl.value.paused
  }
  currentQuality.value = size
}

function onPlaybackRateSelect(rate: number) {
  currentPlaybackRate.value = rate
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

function onLoadStart() {
  isVideoLoaded.value = false
  isBuffering.value = true
}

function onLoadedData() {
  isVideoLoaded.value = true
  isBuffering.value = false
}

function onWaiting() {
  isBuffering.value = true
}

function onPlaying() {
  isBuffering.value = false
}

function onCanPlay() {
  isBuffering.value = false
}

function onSeeking() {
  if (videoEl.value && videoEl.value.readyState < 3) {
    isBuffering.value = true
  }
}

function onSeeked() {
  if (videoEl.value && videoEl.value.readyState >= 3) {
    isBuffering.value = false
  }
}

function onError() {
  isVideoLoaded.value = true
  isBuffering.value = false
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

function playVideo() {
  if (videoEl.value && props.active) {
    videoEl.value.play().catch((err) => {
      console.warn('Playback failed or was blocked by browser:', err)
    })
  }
}

watch(
  videoUrl,
  (newUrl) => {
    bufferedRanges.value = []
    if (newUrl) {
      isVideoLoaded.value = false
      isBuffering.value = true
    }
    nextTick(() => {
      if (videoEl.value) {
        const shouldPlay =
          props.active &&
          (isPlayingOnQualityChange.value !== null
            ? isPlayingOnQualityChange.value
            : props.autoplay)

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

watch(
  () => props.active,
  (isActive) => {
    if (!isActive) {
      if (videoEl.value && !videoEl.value.paused) {
        videoEl.value.pause()
      }
    } else {
      if (videoEl.value && props.autoplay && !isPlaying.value) {
        playVideo()
      }
    }
  },
)

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

watch(
  () => props.mediaItemId,
  () => {
    isVideoLoaded.value = false
    isBuffering.value = true
  },
)

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
  isBuffering.value = false
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

function onTimeUpdate() {
  if (videoEl.value && videoEl.value.paused) {
    currentTime.value = videoEl.value.currentTime
  }
  updateBufferedProgress()
}

function onProgress() {
  updateBufferedProgress()
}

function togglePlay(showOverlay = false) {
  if (!videoEl.value || !props.active) return
  if (videoEl.value.paused) {
    playVideo()
    if (showOverlay) triggerOverlay('play')
  } else {
    videoEl.value.pause()
    if (showOverlay) triggerOverlay('pause')
  }
}

function seekBy(seconds: number) {
  if (!videoEl.value || !props.active) return
  let target = videoEl.value.currentTime + seconds
  if (target < 0) target = 0
  if (target > duration.value) target = duration.value
  videoEl.value.currentTime = target
  currentTime.value = target
}

function stepFrame(direction: number) {
  if (!videoEl.value || !props.active) return
  const frameDuration = 1 / fps.value
  let target = videoEl.value.currentTime + direction * frameDuration
  if (target < 0) target = 0
  if (target > duration.value) target = duration.value
  videoEl.value.currentTime = target
  currentTime.value = target
}

function onSeekInput(val: number) {
  if (videoEl.value && props.active) {
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

function adjustVolume(amount: number) {
  let target = savedVolume.value + amount
  if (target < 0) target = 0
  if (target > 1) target = 1
  savedVolume.value = parseFloat(target.toFixed(2))
  if (target > 0) {
    isMuted.value = false
  }
}

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

function handleKeyDown(e: KeyboardEvent) {
  if (!props.active) return
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

onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
  if (videoEl.value && isPlaying.value === false && props.autoplay && props.active) {
    playVideo()
  }
})

onUnmounted(() => {
  emit('buffering', false)
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
  }
})

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
      :style="{ opacity: isVideoLoaded || !showBlank ? 1 : 0 }"
      loop
      playsinline
      @loadstart="onLoadStart"
      @loadeddata="onLoadedData"
      @loadedmetadata="onLoadedMetadata"
      @waiting="onWaiting"
      @playing="onPlaying"
      @canplay="onCanPlay"
      @seeking="onSeeking"
      @seeked="onSeeked"
      @error="onError"
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

    <!-- Buffering Spinner Indicator -->
    <div v-if="isBufferingOrLoading" class="buffering-spinner">
      <v-progress-circular indeterminate size="56" width="4" color="white" />
    </div>

    <!-- Center Play/Pause Ripple Indicator -->
    <VideoPlayPauseOverlay :action="overlayAction" :trigger="overlayTrigger" />

    <!-- Custom Player Controls Bar -->
    <VideoControls
      v-if="active"
      :current-time="currentTime"
      :duration="duration"
      :buffered="bufferedRanges"
      :is-playing="isPlaying"
      :is-muted="isMuted"
      :volume="savedVolume"
      :show-ui="showUi"
      :is-fullscreen="isFullscreen"
      :quality="currentQuality"
      :sorted-video-sizes="sortedVideoSizes"
      :source-height="sourceHeight"
      :is-source-available="isSourceAvailable"
      :has-thumbnails="hasThumbnails"
      :current-playback-rate="currentPlaybackRate"
      :playback-rates="playbackRates"
      :attach-fullscreen="elementalFullscreen && isFullscreen"
      @toggle-play="togglePlay()"
      @toggle-mute="toggleMute"
      @toggle-fullscreen="toggleFullscreen"
      @seek="onSeekInput"
      @update:volume="onVolumeChange"
      @update:quality="onQualitySelect"
      @update:playback-rate="onPlaybackRateSelect"
    />
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

.buffering-spinner {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 1500;
  pointer-events: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.buffering-spinner :deep(.v-progress-circular) {
  filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.4));
}
</style>
