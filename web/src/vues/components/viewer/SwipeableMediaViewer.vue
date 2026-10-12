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
    orderedIds?: string[]
    prevId?: string | null
    nextId?: string | null
    isVideo: boolean
    disableEventCapture?: boolean
    muted?: boolean
    showUi?: boolean
    autoplay?: boolean
    elementalFullscreen?: boolean
    forcePano?: PannellumConfig | undefined
    photoGalleryHeight?: number
    showGallery?: boolean
  }>(),
  {
    orderedIds: () => [],
    prevId: null,
    nextId: null,
    disableEventCapture: false,
    muted: false,
    showUi: true,
    autoplay: true,
    elementalFullscreen: false,
    forcePano: undefined,
    photoGalleryHeight: 100,
    showGallery: false,
  },
)

const emit = defineEmits<{
  (e: 'next'): void
  (e: 'prev'): void
  (e: 'change-media', mediaId: string): void
  (e: 'zoom-change', isZoomed: boolean): void
  (e: 'pano-active', isActive: boolean): void
  (e: 'buffering', isBuffering: boolean): void
}>()

const mediaItemStore = useMediaItemStore()
const viewPhotoStore = useViewPhotoStore()

const containerRef = useTemplateRef('containerRef')
const trackRef = useTemplateRef('trackRef')
const { width: containerWidth } = useElementSize(containerRef)

// Local active item tracking to decouple rapid swipe transitions from router latency
const activeId = ref(props.mediaItemId)

// Synchronize if activeId was changed from outside (e.g., bottom gallery click)
watch(
  () => props.mediaItemId,
  (newId) => {
    if (newId && newId !== activeId.value) {
      activeId.value = newId
      dragOffset.value = 0
      isDragging.value = false
      isAnimating.value = false
      pendingAction = null
      isZoomed.value = false
    }
  },
)

const currentIndex = computed(() => {
  if (props.orderedIds && props.orderedIds.length > 0) {
    return props.orderedIds.indexOf(activeId.value)
  }
  return -1
})

const effectivePrevId = computed(() => {
  if (props.orderedIds && props.orderedIds.length > 0) {
    const idx = currentIndex.value
    if (idx <= 0) return null
    return props.orderedIds[idx - 1] ?? null
  }
  return props.prevId
})

const effectiveNextId = computed(() => {
  if (props.orderedIds && props.orderedIds.length > 0) {
    const idx = currentIndex.value
    if (idx === -1 || idx >= props.orderedIds.length - 1) return null
    return props.orderedIds[idx + 1] ?? null
  }
  return props.nextId
})

function isItemVideo(itemId: string | null | undefined): boolean {
  if (!itemId) return false
  const full = mediaItemStore.anyMediaItems.get(itemId)
  if (full !== undefined) return full.is_video
  const meta = viewPhotoStore.idsMetadata.get(itemId)
  if (meta !== undefined) return meta.isVideo
  return false
}

const isCurrentVideo = computed(() => isItemVideo(activeId.value))
const isPrevVideo = computed(() => isItemVideo(effectivePrevId.value))
const isNextVideo = computed(() => isItemVideo(effectiveNextId.value))

// Viewer states
const isZoomed = ref(false)
const isPanoActive = ref(false)

// Touch / Swipe states
const isDragging = ref(false)
const isAnimating = ref(false)
const dragOffset = ref(0)
let pendingAction: 'next' | 'prev' | null = null
let animationTimeoutId: ReturnType<typeof setTimeout> | null = null

// Touch tracking metrics
let startX = 0
let startY = 0
let lastX = 0
let lastTime = 0
let velocityX = 0
let isMultiTouch = false
let gestureDirection: 'horizontal' | 'vertical' | null = null

const trackStyle = computed(() => {
  const width = containerWidth.value || window.innerWidth
  const currentTranslateX = -width + dragOffset.value

  return {
    transform: `translate3d(${currentTranslateX}px, 0, 0)`,
    transition: isAnimating.value ? 'transform 0.22s cubic-bezier(0.2, 0.9, 0.3, 1)' : 'none',
    willChange: isDragging.value || isAnimating.value ? 'transform' : 'auto',
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

function clearAnimationTimeout() {
  if (animationTimeoutId !== null) {
    clearTimeout(animationTimeoutId)
    animationTimeoutId = null
  }
}

/**
 * If the user touches the screen while the previous swipe is still animating,
 * immediately settle on the destination photo so the next gesture can be accepted.
 */
function resolveAnimationImmediately() {
  clearAnimationTimeout()
  isAnimating.value = false

  if (pendingAction === 'next' && effectiveNextId.value) {
    const targetId = effectiveNextId.value
    activeId.value = targetId
    dragOffset.value = 0
    pendingAction = null
    emit('change-media', targetId)
  } else if (pendingAction === 'prev' && effectivePrevId.value) {
    const targetId = effectivePrevId.value
    activeId.value = targetId
    dragOffset.value = 0
    pendingAction = null
    emit('change-media', targetId)
  } else {
    dragOffset.value = 0
    pendingAction = null
  }
}

function onTouchStart(e: TouchEvent) {
  if (props.disableEventCapture || isZoomed.value || isPanoActive.value) {
    return
  }

  // Interrupted animation: settle instantly and capture this new touch
  if (isAnimating.value) {
    resolveAnimationImmediately()
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
  if (isMultiTouch || props.disableEventCapture || isZoomed.value || isPanoActive.value) {
    return
  }

  if (e.touches.length > 1) {
    isMultiTouch = true
    isDragging.value = false
    dragOffset.value = 0
    return
  }

  const touch = e.touches[0]
  const rawDeltaX = touch.clientX - startX
  const rawDeltaY = touch.clientY - startY

  // Disambiguate horizontal swipe vs vertical scroll
  if (gestureDirection === null) {
    const absX = Math.abs(rawDeltaX)
    const absY = Math.abs(rawDeltaY)

    if (absX < 8 && absY < 8) return

    if (absX >= absY) {
      gestureDirection = 'horizontal'
      isDragging.value = true
      // Reset startX to current touch position so movement starts seamlessly at 0px without jumping
      startX = touch.clientX
      lastX = touch.clientX
      lastTime = performance.now()
      dragOffset.value = 0
      return
    } else {
      gestureDirection = 'vertical'
      return
    }
  }

  if (gestureDirection === 'horizontal') {
    if (e.cancelable) e.preventDefault()

    const deltaX = touch.clientX - startX
    const now = performance.now()
    const dt = now - lastTime
    if (dt > 10) {
      velocityX = (touch.clientX - lastX) / dt
      lastX = touch.clientX
      lastTime = now
    }

    // Apply edge resistance (rubber-banding) at boundaries
    if (deltaX > 0 && effectivePrevId.value === null) {
      dragOffset.value = deltaX * 0.25
    } else if (deltaX < 0 && effectiveNextId.value === null) {
      dragOffset.value = deltaX * 0.25
    } else {
      dragOffset.value = deltaX
    }
  }
}

function onTouchEnd() {
  if (!isDragging.value) return

  isDragging.value = false
  const width = containerWidth.value || window.innerWidth
  const distanceThreshold = width * 0.2
  const flickThreshold = 0.35

  const isFlickNext = velocityX < -flickThreshold && dragOffset.value < 0
  const isFlickPrev = velocityX > flickThreshold && dragOffset.value > 0

  const shouldGoNext =
    effectiveNextId.value !== null && (dragOffset.value < -distanceThreshold || isFlickNext)
  const shouldGoPrev =
    effectivePrevId.value !== null && (dragOffset.value > distanceThreshold || isFlickPrev)

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
  }, 260)
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

  if (action === 'next' && effectiveNextId.value) {
    const targetId = effectiveNextId.value
    activeId.value = targetId
    dragOffset.value = 0
    pendingAction = null
    emit('change-media', targetId)
  } else if (action === 'prev' && effectivePrevId.value) {
    const targetId = effectivePrevId.value
    activeId.value = targetId
    dragOffset.value = 0
    pendingAction = null
    emit('change-media', targetId)
  } else {
    dragOffset.value = 0
    pendingAction = null
  }
}

onBeforeUnmount(() => {
  clearAnimationTimeout()
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
          v-if="effectivePrevId"
          :media-item-id="effectivePrevId"
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
          :media-item-id="activeId"
          :active="true"
          :is-video="isCurrentVideo"
          :show-ui="showUi"
          :muted="muted"
          :autoplay="autoplay"
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
          v-if="effectiveNextId"
          :media-item-id="effectiveNextId"
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
}

.swipe-slide {
  width: 33.333333%;
  height: 100%;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
  contain: strict;
}
</style>
