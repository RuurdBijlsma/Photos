<script setup lang="ts">
import { nextTick, onUnmounted, ref, useTemplateRef, watch } from 'vue'
import { useDelayedBoolean } from '@/scripts/composables/useDelayedBoolean.ts'

defineOptions({
  inheritAttrs: false,
})

const props = withDefaults(
  defineProps<{
    src: string | null
    mediaItemId: string
    muted: boolean
    volume: number
    playbackRate: number
    crossorigin?: 'use-credentials' | undefined
    autoplay?: boolean
    fps?: number
  }>(),
  {
    autoplay: true,
    fps: 30,
    crossorigin: undefined,
  },
)

const emit = defineEmits<{
  (e: 'update:currentTime', time: number): void
  (e: 'update:duration', duration: number): void
  (e: 'update:isPlaying', isPlaying: boolean): void
  (e: 'update:isBuffering', isBuffering: boolean): void
  (e: 'update:isVideoLoaded', isLoaded: boolean): void
  (e: 'update:bufferedRanges', ranges: Array<{ start: number; end: number }>): void
  (e: 'click-video'): void
  (e: 'dblclick-video'): void
}>()

const videoEl = useTemplateRef<HTMLVideoElement>('videoElement')

const isVideoLoaded = ref(false)
const isBuffering = ref(false)
const isPlaying = ref(false)

const timeToRestore = ref<number | null>(null)
const isPlayingOnSourceChange = ref<boolean | null>(null)

// Delayed blanking state for smooth loading transitions
const isPendingLoad = ref(true)
const showBlank = useDelayedBoolean(isPendingLoad, 100)

let animationFrameId: number | null = null

function updateBufferedProgress() {
  if (!videoEl.value) return
  const b = videoEl.value.buffered
  const ranges: Array<{ start: number; end: number }> = []
  for (let i = 0; i < b.length; i++) {
    ranges.push({
      start: b.start(i),
      end: b.end(i),
    })
  }
  emit('update:bufferedRanges', ranges)
}

function updateProgressSmoothly() {
  if (videoEl.value && !videoEl.value.paused) {
    emit('update:currentTime', videoEl.value.currentTime)
    updateBufferedProgress()
    animationFrameId = requestAnimationFrame(updateProgressSmoothly)
  }
}

// Media event handlers
function onLoadStart() {
  isVideoLoaded.value = false
  isPendingLoad.value = true
  isBuffering.value = true
  emit('update:isVideoLoaded', false)
  emit('update:isBuffering', true)
}

function onLoadedData() {
  isVideoLoaded.value = true
  isPendingLoad.value = false
  isBuffering.value = false
  emit('update:isVideoLoaded', true)
  emit('update:isBuffering', false)
}

function onLoadedMetadata() {
  if (!videoEl.value) return
  const duration = videoEl.value.duration || 0
  emit('update:duration', duration)

  if (timeToRestore.value !== null) {
    videoEl.value.currentTime = timeToRestore.value
    emit('update:currentTime', timeToRestore.value)
    timeToRestore.value = null
  }

  videoEl.value.playbackRate = props.playbackRate
  videoEl.value.volume = props.volume
  videoEl.value.muted = props.muted
  updateBufferedProgress()
}

function onWaiting() {
  isBuffering.value = true
  emit('update:isBuffering', true)
}

function onPlaying() {
  isBuffering.value = false
  emit('update:isBuffering', false)
}

function onCanPlay() {
  isBuffering.value = false
  emit('update:isBuffering', false)
}

function onSeeking() {
  if (videoEl.value && videoEl.value.readyState < 3) {
    isBuffering.value = true
    emit('update:isBuffering', true)
  }
}

function onSeeked() {
  if (videoEl.value && videoEl.value.readyState >= 3) {
    isBuffering.value = false
    emit('update:isBuffering', false)
  }
}

function onError() {
  isVideoLoaded.value = true
  isPendingLoad.value = false
  isBuffering.value = false
  emit('update:isVideoLoaded', true)
  emit('update:isBuffering', false)
}

function onPlay() {
  isPlaying.value = true
  isBuffering.value = false
  emit('update:isPlaying', true)
  emit('update:isBuffering', false)
  if (animationFrameId === null) {
    animationFrameId = requestAnimationFrame(updateProgressSmoothly)
  }
}

function onPause() {
  isPlaying.value = false
  emit('update:isPlaying', false)
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }
}

function onTimeUpdate() {
  if (videoEl.value && videoEl.value.paused) {
    emit('update:currentTime', videoEl.value.currentTime)
  }
  updateBufferedProgress()
}

function onProgress() {
  updateBufferedProgress()
}

// Playback actions
function playVideo() {
  if (videoEl.value) {
    videoEl.value.play().catch((err) => {
      console.warn('Playback failed or was blocked by browser:', err)
    })
  }
}

function play() {
  playVideo()
}

function pause() {
  videoEl.value?.pause()
}

function togglePlay() {
  if (!videoEl.value) return
  if (videoEl.value.paused) {
    playVideo()
  } else {
    videoEl.value.pause()
  }
}

function seek(time: number) {
  if (!videoEl.value) return
  videoEl.value.currentTime = time
  emit('update:currentTime', time)
}

function seekBy(seconds: number) {
  if (!videoEl.value) return
  const max = videoEl.value.duration || 0
  let target = videoEl.value.currentTime + seconds
  if (target < 0) target = 0
  if (max > 0 && target > max) target = max
  videoEl.value.currentTime = target
  emit('update:currentTime', target)
}

function stepFrame(direction: number) {
  if (!videoEl.value) return
  const frameDuration = 1 / props.fps
  const max = videoEl.value.duration || 0
  let target = videoEl.value.currentTime + direction * frameDuration
  if (target < 0) target = 0
  if (max > 0 && target > max) target = max
  videoEl.value.currentTime = target
  emit('update:currentTime', target)
}

// Watch props to sync directly with native element
watch(
  () => props.volume,
  (val) => {
    if (videoEl.value) videoEl.value.volume = val
  },
  { immediate: true },
)

watch(
  () => props.muted,
  (val) => {
    if (videoEl.value) videoEl.value.muted = val
  },
  { immediate: true },
)

watch(
  () => props.playbackRate,
  (val) => {
    if (videoEl.value) videoEl.value.playbackRate = val
  },
  { immediate: true },
)

// Handle source changes and preserve timestamps across quality switches
watch(
  [() => props.src, () => props.mediaItemId],
  ([newSrc, newMediaId], [, oldMediaId]) => {
    const isSameItem = newMediaId === oldMediaId
    emit('update:bufferedRanges', [])

    if (isSameItem && videoEl.value) {
      timeToRestore.value = videoEl.value.currentTime
      isPlayingOnSourceChange.value = !videoEl.value.paused
    } else {
      timeToRestore.value = null
      isPlayingOnSourceChange.value = null
      emit('update:currentTime', 0)
    }

    if (newSrc) {
      isVideoLoaded.value = false
      isPendingLoad.value = true
      isBuffering.value = true
      emit('update:isVideoLoaded', false)
      emit('update:isBuffering', true)
    }

    nextTick(() => {
      if (videoEl.value) {
        const shouldPlay =
          isPlayingOnSourceChange.value !== null ? isPlayingOnSourceChange.value : props.autoplay

        videoEl.value.load()

        if (shouldPlay) {
          playVideo()
        } else {
          videoEl.value.pause()
        }
        videoEl.value.playbackRate = props.playbackRate
        isPlayingOnSourceChange.value = null
      }
    })
  },
  { immediate: true },
)

onUnmounted(() => {
  if (animationFrameId !== null) {
    cancelAnimationFrame(animationFrameId)
    animationFrameId = null
  }
})

defineExpose({
  play,
  pause,
  togglePlay,
  seek,
  seekBy,
  stepFrame,
})
</script>

<template>
  <video
    v-if="src"
    ref="videoElement"
    class="video-element"
    :src="src"
    :muted="muted"
    :crossorigin="crossorigin"
    :style="{
      opacity: isVideoLoaded || !showBlank ? 1 : 0,
    }"
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
    @click="emit('click-video')"
    @dblclick="emit('dblclick-video')"
  />
</template>

<style scoped>
.video-element {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
</style>
