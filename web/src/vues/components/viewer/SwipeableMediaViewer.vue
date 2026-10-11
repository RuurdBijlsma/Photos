<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue'
import { useElementSize } from '@vueuse/core'
import MediaViewer from '@/vues/components/viewer/MediaViewer.vue'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'

const props = withDefaults(
  defineProps<{
    mediaItemId: string
    prevId?: string | null
    nextId?: string | null
    isVideo: boolean
    disableEventCapture?: boolean
    muted?: boolean
    showUi?: boolean
    elementalFullscreen?: boolean
    forcePano?: PannellumConfig | undefined
    photoGalleryHeight?: number
    showGallery?: boolean
  }>(),
  {
    prevId: null,
    nextId: null,
    disableEventCapture: false,
    muted: false,
    showUi: true,
    elementalFullscreen: false,
    forcePano: undefined,
    photoGalleryHeight: 100,
    showGallery: false,
  },
)

const emit = defineEmits<{
  (e: 'next'): void
  (e: 'prev'): void
  (e: 'zoom-change', isZoomed: boolean): void
  (e: 'pano-active', isActive: boolean): void
  (e: 'buffering', isBuffering: boolean): void
}>()

const mediaItemStore = useMediaItemStore()
const viewPhotoStore = useViewPhotoStore()

const containerRef = useTemplateRef('containerRef')
const trackRef = useTemplateRef('trackRef')
const { width: containerWidth } = useElementSize(containerRef)

// Viewer states
const isZoomed = ref(false)
const isPanoActive = ref(false)

// Touch / Swipe states
const isDragging = ref(false)
const isAnimating = ref(false)
const dragOffset = ref(0) // Drag offset in pixels
let pendingAction: 'next' | 'prev' | null = null
let animationTimeoutId: ReturnType<typeof setTimeout> | null = null
let navFallbackTimer: ReturnType<typeof setTimeout> | null = null

// Touch tracking metrics
let startX = 0
let startY = 0
let lastX = 0
let lastTime = 0
let velocityX = 0
let isMultiTouch = false
let gestureDirection: 'horizontal' | 'vertical' | null = null

function isItemVideo(itemId: string | null | undefined): boolean {
  if (!itemId) return false
  const full = mediaItemStore.anyMediaItems.get(itemId)
  if (full !== undefined) return full.is_video
  const meta = viewPhotoStore.idsMetadata.get(itemId)
  if (meta !== undefined) return meta.isVideo
  return false
}

const isPrevVideo = computed(() => isItemVideo(props.prevId))
const isNextVideo = computed(() => isItemVideo(props.nextId))

// Current track transform: resting position is -containerWidth (centered on Slot 0)
const trackStyle = computed(() => {
  const width = containerWidth.value || window.innerWidth
  const currentTranslateX = -width + dragOffset.value

  return {
    transform: `translate3d(${currentTranslateX}px, 0, 0)`,
    transition: isAnimating.value ? 'transform 0.28s cubic-bezier(0.25, 1, 0.5, 1)' : 'none',
  }
})

function onZoomChange(zoomed: boolean) {
  isZoomed.value = zoomed
  emit('zoom-change', zoomed)
}

function onPanoActive(active: boolean) {
  isPanoActive.value = active
  emit('pano-active', active)
}

function clearNavFallback() {
  if (navFallbackTimer !== null) {
    clearTimeout(navFallbackTimer)
    navFallbackTimer = null
  }
}

function clearAnimationTimeout() {
  if (animationTimeoutId !== null) {
    clearTimeout(animationTimeoutId)
    animationTimeoutId = null
  }
}

function onTouchStart(e: TouchEvent) {
  if (
    props.disableEventCapture ||
    isZoomed.value ||
    isPanoActive.value ||
    isAnimating.value ||
    pendingAction !== null
  ) {
    return
  }

  if (e.touches.length > 1) {
    isMultiTouch = true
    if (isDragging.value) {
      isDragging.value = false
      dragOffset.value = 0
    }
    return
  }

  isMultiTouch = false
  gestureDirection = null
  const touch = e.touches[0]
  startX = touch.clientX
  startY = touch.clientY
  lastX = touch.clientX
  lastTime = performance.now()
  velocityX = 0
}

function onTouchMove(e: TouchEvent) {
  if (
    isMultiTouch ||
    props.disableEventCapture ||
    isZoomed.value ||
    isPanoActive.value ||
    isAnimating.value ||
    pendingAction !== null
  ) {
    return
  }

  if (e.touches.length > 1) {
    isMultiTouch = true
    isDragging.value = false
    dragOffset.value = 0
    return
  }

  const touch = e.touches[0]
  const deltaX = touch.clientX - startX
  const deltaY = touch.clientY - startY

  // Disambiguate horizontal swipe vs vertical scroll
  if (gestureDirection === null) {
    const absX = Math.abs(deltaX)
    const absY = Math.abs(deltaY)

    if (absX < 8 && absY < 8) return

    if (absX >= absY) {
      gestureDirection = 'horizontal'
      isDragging.value = true
    } else {
      gestureDirection = 'vertical'
      return
    }
  }

  if (gestureDirection === 'horizontal') {
    if (e.cancelable) e.preventDefault()

    const now = performance.now()
    const dt = now - lastTime
    if (dt > 10) {
      velocityX = (touch.clientX - lastX) / dt
      lastX = touch.clientX
      lastTime = now
    }

    // Apply edge resistance (rubber-banding) at boundaries
    if (deltaX > 0 && props.prevId === null) {
      dragOffset.value = deltaX * 0.25
    } else if (deltaX < 0 && props.nextId === null) {
      dragOffset.value = deltaX * 0.25
    } else {
      dragOffset.value = deltaX
    }
  }
}

function onTouchEnd() {
  if (!isDragging.value || isAnimating.value) {
    isDragging.value = false
    return
  }

  isDragging.value = false
  const width = containerWidth.value || window.innerWidth
  const distanceThreshold = width * 0.2
  const flickThreshold = 0.35

  const isFlickNext = velocityX < -flickThreshold && dragOffset.value < 0
  const isFlickPrev = velocityX > flickThreshold && dragOffset.value > 0

  const shouldGoNext =
    props.nextId !== null && (dragOffset.value < -distanceThreshold || isFlickNext)
  const shouldGoPrev =
    props.prevId !== null && (dragOffset.value > distanceThreshold || isFlickPrev)

  if (shouldGoNext) {
    pendingAction = 'next'
    animateTo(-width)
  } else if (shouldGoPrev) {
    pendingAction = 'prev'
    animateTo(width)
  } else {
    pendingAction = null
    animateTo(0)
  }
}

function onTouchCancel() {
  if (isDragging.value) {
    isDragging.value = false
    animateTo(0)
  }
}

function animateTo(targetOffset: number) {
  isAnimating.value = true
  dragOffset.value = targetOffset

  clearAnimationTimeout()
  animationTimeoutId = setTimeout(() => {
    finishTransition()
  }, 320)
}

function onTransitionEnd(e: TransitionEvent) {
  if (e.target !== trackRef.value) return
  finishTransition()
}

function finishTransition() {
  clearAnimationTimeout()
  if (!isAnimating.value) return

  isAnimating.value = false
  const action = pendingAction

  if (action === 'next') {
    // Keep dragOffset at -width so Photo B stays on screen while router updates
    clearNavFallback()
    navFallbackTimer = setTimeout(() => {
      // Safety fallback in case router navigation fails or is blocked
      dragOffset.value = 0
      pendingAction = null
    }, 800)
    emit('next')
  } else if (action === 'prev') {
    // Keep dragOffset at +width so Photo Prev stays on screen while router updates
    clearNavFallback()
    navFallbackTimer = setTimeout(() => {
      dragOffset.value = 0
      pendingAction = null
    }, 800)
    emit('prev')
  } else {
    // Snapped back to current item
    dragOffset.value = 0
    pendingAction = null
  }
}

// Reset internal positions once active item actually updates
watch(
  () => props.mediaItemId,
  () => {
    clearNavFallback()
    dragOffset.value = 0
    isDragging.value = false
    isAnimating.value = false
    pendingAction = null
    isZoomed.value = false
  },
)

onBeforeUnmount(() => {
  clearAnimationTimeout()
  clearNavFallback()
})
</script>

<template>
  <div
    ref="containerRef"
    class="swipeable-media-viewer"
    :class="{ 'is-dragging': isDragging }"
    :style="{
      height: `calc(100% - ${showGallery ? photoGalleryHeight : 0}px)`,
    }"
    @touchstart="onTouchStart"
    @touchmove="onTouchMove"
    @touchend="onTouchEnd"
    @touchcancel="onTouchCancel"
  >
    <div ref="trackRef" class="swipe-track" :style="trackStyle" @transitionend="onTransitionEnd">
      <!-- Slot -1: Previous Item -->
      <div class="swipe-slide">
        <MediaViewer
          v-if="prevId"
          :media-item-id="prevId"
          :active="false"
          :is-video="isPrevVideo"
          :show-ui="showUi"
          :muted="true"
          :disable-event-capture="true"
          :elemental-fullscreen="false"
        />
      </div>

      <!-- Slot 0: Active / Current Item -->
      <div class="swipe-slide">
        <MediaViewer
          :media-item-id="mediaItemId"
          :active="true"
          :is-video="isVideo"
          :show-ui="showUi"
          :muted="muted"
          :elemental-fullscreen="elementalFullscreen"
          :force-pano="forcePano"
          :disable-event-capture="disableEventCapture"
          @zoom-change="onZoomChange"
          @pano-active="onPanoActive"
          @buffering="emit('buffering', $event)"
        />
      </div>

      <!-- Slot +1: Next Item -->
      <div class="swipe-slide">
        <MediaViewer
          v-if="nextId"
          :media-item-id="nextId"
          :active="false"
          :is-video="isNextVideo"
          :show-ui="showUi"
          :muted="true"
          :disable-event-capture="true"
          :elemental-fullscreen="false"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.swipeable-media-viewer {
  width: 100%;
  position: absolute;
  top: 0;
  left: 0;
  z-index: 1500;
  overflow: hidden;
  touch-action: pan-y;
  user-select: none;
  background-color: rgb(var(--bg));
}

.swipeable-media-viewer.is-dragging {
  touch-action: none;
}

.swipe-track {
  display: flex;
  width: 300%;
  height: 100%;
  will-change: transform;
}

.swipe-slide {
  width: 33.333333%;
  height: 100%;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
}
</style>
