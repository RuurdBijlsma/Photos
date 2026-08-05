<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onUnmounted, ref, watch } from 'vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import axios from 'axios'
import { useElementSize, useEventListener, useRafFn, useTimeoutFn } from '@vueuse/core'
import { SERVER_BASE_URL } from '@/scripts/services/api.ts'
import type { FullMediaItem } from '@/scripts/types/api/fullPhoto.ts'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

const PanoramaViewer = defineAsyncComponent(
  () => import('@/vues/components/viewer/components/PanoramaViewer.vue'),
)

const props = withDefaults(
  defineProps<{
    disableEventCapture: boolean
    mediaItemId: string
    showUi?: boolean
    forcePano?: PannellumConfig | undefined
  }>(),
  {
    showUi: true,
  },
)

const emit = defineEmits<{
  (e: 'zoom-change', isZoomed: boolean): void
  (e: 'pano-active', isActive: boolean): void
}>()

const mediaItemStore = useMediaItemStore()
const settings = useSettingStore()
const viewPhotoStore = useViewPhotoStore()
const authStore = useAuthStore()

// Zoom and Pan state
const scale = ref(1)
const translateX = ref(0)
const translateY = ref(0)
const thumbErrored = ref(false)
const containerRef = ref<HTMLElement | null>(null)
const { width: containerWidth, height: containerHeight } = useElementSize(containerRef)
const thumbRef = ref<HTMLImageElement | null>(null)
const baseUrl = SERVER_BASE_URL

const activePointers = new Map<number, PointerEvent>()
let initialPinchDistance = 0
let initialScale = 1
let isDragging = false
let startX = 0
let startY = 0
let startTranslateX = 0
let startTranslateY = 0

const fullImage = computed(() => mediaItemStore.anyMediaItems.get(props.mediaItemId))
const generatedThumbsAvailable = computed(() => fullImage.value?.has_thumbnails ?? true)

// Panorama states
const isPanorama = computed(
  () => props.forcePano !== undefined || (fullImage.value?.use_panorama_viewer ?? false),
)
const panoramaConfig = computed(() => props.forcePano ?? fullImage.value?.panorama_config)
const is3DMode = ref(false)

// Immediate Thumbnail URL (1440p)
const imageUrl = computed(() => {
  return mediaItemService.getPhotoThumbnail(
    props.mediaItemId,
    1440,
    !generatedThumbsAvailable.value,
  )
})

// Full Resolution Background Load
const fullResUrl = ref<string | null>(null)
const fullResLoaded = ref(false)
const isLoadingFull = ref(false)

let currentAbortController: AbortController | null = null

// Native format support check
function isMimeTypeSupported(mimeType?: string): boolean {
  if (!mimeType) return false
  const lower = mimeType.toLowerCase()
  // Standard formats supported by all modern browsers
  if (
    lower === 'image/jpeg' ||
    lower === 'image/jpg' ||
    lower === 'image/png' ||
    lower === 'image/webp' ||
    lower === 'image/gif' ||
    lower === 'image/avif'
  ) {
    return true
  }
  // HEIC support check (natively supported on Apple devices/Safari)
  if (lower === 'image/heic' || lower === 'image/heif') {
    return /^((?!chrome|android).)*safari/i.test(navigator.userAgent)
  }
  return false
}

// Cleanup full resolution load resources
function cleanup() {
  if (currentAbortController) {
    currentAbortController.abort()
    currentAbortController = null
  }
  if (fullResUrl.value) {
    URL.revokeObjectURL(fullResUrl.value)
    fullResUrl.value = null
  }
  fullResLoaded.value = false
  isLoadingFull.value = false
}

// Download the original full-resolution media file blob
async function loadFullResBlob(item: FullMediaItem) {
  const controller = new AbortController()
  currentAbortController = controller
  isLoadingFull.value = true

  try {
    const res = await mediaItemService.downloadMediaFileById(item.id, controller.signal)

    if (controller.signal.aborted || props.mediaItemId !== item.id) {
      return
    }

    const objectUrl = URL.createObjectURL(res.data)
    fullResUrl.value = objectUrl
  } catch (err: unknown) {
    const isAbortError =
      err instanceof DOMException && (err.name === 'CanceledError' || err.name === 'AbortError')

    if (isAbortError || axios.isCancel(err) || controller.signal.aborted) {
      console.log('Background fetch aborted for image:', item.id)
    } else {
      console.error('Failed to load full-resolution image:', err)
    }
  } finally {
    if (currentAbortController === controller) {
      isLoadingFull.value = false
    }
  }
}

// Async decoding to prevent main thread "image decode" frame drops during zoom
async function onFullResLoad(e: Event) {
  const img = e.target as HTMLImageElement
  try {
    await img.decode()
    fullResLoaded.value = true
  } catch (err) {
    console.warn('Background image decoding failed, displaying immediately:', err)
    fullResLoaded.value = true
  }
}

// Motion Photo Settings & Logic
const showingMotionVideo = ref(false)
const videoPlayerRef = ref<HTMLVideoElement | null>(null)
const motionVideoUrl = computed(() => mediaItemService.getMotionVideo(props.mediaItemId))

const { pause: stopMonitoring, resume: startMonitoring } = useRafFn(
  () => {
    const video = videoPlayerRef.value
    if (!video) {
      stopMonitoring()
      return
    }

    const presentationTimestampUs =
      fullImage.value?.media_features?.motion_photo_presentation_timestamp
    let targetTime = 0

    if (presentationTimestampUs !== undefined && presentationTimestampUs > 0) {
      targetTime = presentationTimestampUs / 1000000
    } else if (video.duration && !isNaN(video.duration)) {
      targetTime = video.duration / 2
    }

    // Swap back to the static photo once the presentation timestamp is crossed
    if (targetTime > 0 && video.currentTime >= targetTime) {
      showingMotionVideo.value = false
      stopMonitoring()
    }
  },
  { immediate: false },
)

function playMotionPhoto() {
  if (!settings.playMotionPhotos) return
  showingMotionVideo.value = true
  nextTick(() => {
    if (videoPlayerRef.value) {
      videoPlayerRef.value.currentTime = 0
      videoPlayerRef.value.play().catch((err) => {
        console.error('Failed to autoplay motion photo:', err)
      })
    }
  })
}

function onVideoEnded() {
  showingMotionVideo.value = false
  stopMonitoring()
}

// Reset UI viewport state whenever media item ID changes
watch(
  () => props.mediaItemId,
  () => {
    scale.value = 1
    translateX.value = 0
    translateY.value = 0
    is3DMode.value = false
    thumbErrored.value = false
    showingMotionVideo.value = false
    stopMonitoring()
  },
  { immediate: true },
)

// Drive Media Content Loading Lifecycle whenever fullImage changes
watch(
  fullImage,
  (item) => {
    // Always clean up previous blob URLs or active network requests first
    cleanup()

    if (!item) return

    if (settings.playMotionPhotos && item.media_features?.is_motion_photo) {
      playMotionPhoto()
    }

    if (isMimeTypeSupported(item.media_features?.mime_type) && authStore.isAuthenticated) {
      loadFullResBlob(item as FullMediaItem)
    }
  },
  { immediate: true },
)

watch(
  is3DMode,
  (isActive) => {
    emit('pano-active', isActive)
  },
  { immediate: true },
)

const rotation = computed(() => viewPhotoStore.rotatedPhotos.get(props.mediaItemId) ?? 0)
const isRotated90or270 = computed(() => {
  const norm = ((rotation.value % 360) + 360) % 360
  return norm === 90 || norm === 270
})

// Reset zoom when client rotation is active
watch(
  rotation,
  (newRot) => {
    if (newRot % 360 !== 0) {
      scale.value = 1
      translateX.value = 0
      translateY.value = 0
    }
  },
  { immediate: true },
)

// Emit zoom state to parents
watch(
  [scale, is3DMode, rotation],
  ([newScale, is3D, rot]) => {
    emit('zoom-change', !is3D && rot % 360 === 0 && newScale > 1)
  },
  { immediate: true },
)

// Listen to motion trigger from ViewPhoto top right menu
watch(
  () => viewPhotoStore.playMotionTrigger,
  () => {
    if (fullImage.value?.media_features?.is_motion_photo) {
      playMotionPhoto()
    }
  },
)

onUnmounted(() => {
  cleanup()
})

const wrapperStyle = computed(() => {
  const normRot = ((rotation.value % 360) + 360) % 360

  if (isRotated90or270.value) {
    const w = containerHeight.value || containerRef.value?.clientHeight || 0
    const h = containerWidth.value || containerRef.value?.clientWidth || 0
    return {
      width: w > 0 ? `${w}px` : '100vh',
      height: h > 0 ? `${h}px` : '100vw',
      position: 'absolute' as const,
      top: '50%',
      left: '50%',
      transform: `translate(-50%, -50%) rotate(${normRot}deg)`,
      transformOrigin: 'center center',
    }
  }

  if (normRot === 180) {
    return {
      width: '100%',
      height: '100%',
      position: 'absolute' as const,
      top: '0px',
      left: '0px',
      transform: 'rotate(180deg)',
      transformOrigin: 'center center',
    }
  }

  return {
    width: '100%',
    height: '100%',
    position: 'absolute' as const,
    top: '0px',
    left: '0px',
    transform: `translate3d(${translateX.value}px, ${translateY.value}px, 0) scale(${scale.value})`,
    transformOrigin: '0 0',
  }
})

// Active Transform state handling using VueUse's lifecycle-aware timer
const isTransforming = ref(false)

const { start: startTransformTimer, stop: stopTransformTimer } = useTimeoutFn(
  () => {
    isTransforming.value = false
  },
  150,
  { immediate: false },
)

function setTransforming(value: boolean) {
  stopTransformTimer()
  isTransforming.value = value
}

function zoomToPoint(clientX: number, clientY: number, newScale: number) {
  if (!containerRef.value || rotation.value % 360 !== 0) return

  const rect = containerRef.value.getBoundingClientRect()
  const xScreen = clientX - rect.left
  const yScreen = clientY - rect.top

  const minScale = 1
  const maxScale = 8
  const clampedScale = Math.max(minScale, Math.min(maxScale, newScale))

  const oldScale = scale.value
  if (oldScale === clampedScale) {
    // We didn't actually change zoom levels (meaning we tried zooming past limits), stop transform immediately
    setTransforming(false)
    return
  }

  // Zoom centered at the cursor coordinates
  const nextTranslateX = xScreen - (xScreen - translateX.value) * (clampedScale / oldScale)
  const nextTranslateY = yScreen - (yScreen - translateY.value) * (clampedScale / oldScale)

  scale.value = clampedScale
  translateX.value = nextTranslateX
  translateY.value = nextTranslateY

  clampTranslations()

  // Snap resolution crispness immediately if we reached minimum or maximum zoom boundaries
  if (clampedScale === minScale || clampedScale === maxScale) {
    setTransforming(false)
  } else {
    setTransforming(true)
    startTransformTimer()
  }
}

function getImageAspectRatio(): number | null {
  if (thumbRef.value && thumbRef.value.naturalWidth > 0 && thumbRef.value.naturalHeight > 0) {
    return thumbRef.value.naturalWidth / thumbRef.value.naturalHeight
  }
  return null
}

function clampTranslations() {
  if (!containerRef.value || rotation.value % 360 !== 0) return

  const w = containerRef.value.clientWidth
  const h = containerRef.value.clientHeight

  const aspectRatio = getImageAspectRatio()
  const containerRatio = w / h

  let drawnWidth = w
  let drawnHeight = h

  if (aspectRatio && aspectRatio > 0) {
    if (aspectRatio > containerRatio) {
      drawnWidth = w
      drawnHeight = w / aspectRatio
    } else {
      drawnWidth = h * aspectRatio
      drawnHeight = h
    }
  }

  const paddingPx = scale.value < 1.4 ? 0 : scale.value * 40

  // Horizontal clamping
  if (scale.value <= 1) {
    translateX.value = 0
  } else if (drawnWidth * scale.value <= w) {
    // Image is narrower than the viewport: Center it, but allow comfortable wiggle room
    const center = (w / 2) * (1 - scale.value)
    const minX = center - paddingPx
    const maxX = center + paddingPx
    translateX.value = Math.max(minX, Math.min(maxX, translateX.value))
  } else {
    // Image is wider than the viewport: Allow panning, clamped with soft edge padding
    const minX = w - paddingPx - ((w + drawnWidth) / 2) * scale.value
    const maxX = paddingPx - ((w - drawnWidth) / 2) * scale.value
    translateX.value = Math.max(minX, Math.min(maxX, translateX.value))
  }

  // Vertical clamping
  if (scale.value <= 1) {
    translateY.value = 0
  } else if (drawnHeight * scale.value <= h) {
    // Image is shorter than the viewport: Center it, but allow comfortable wiggle room
    const center = (h / 2) * (1 - scale.value)
    const minY = center - paddingPx
    const maxY = center + paddingPx
    translateY.value = Math.max(minY, Math.min(maxY, translateY.value))
  } else {
    // Image is taller than the viewport: Allow panning, clamped with soft edge padding
    const minY = h - paddingPx - ((h + drawnHeight) / 2) * scale.value
    const maxY = paddingPx - ((h - drawnHeight) / 2) * scale.value
    translateY.value = Math.max(minY, Math.min(maxY, translateY.value))
  }
}

// Pointer events panning & pinch-to-zoom
function handlePointerDown(e: PointerEvent) {
  if (props.disableEventCapture || rotation.value % 360 !== 0) return
  if (e.pointerType === 'mouse' && e.button !== 0) {
    return
  }

  if (scale.value > 1) {
    setTransforming(true)
  }

  const target = e.currentTarget as HTMLElement
  target.setPointerCapture(e.pointerId)
  activePointers.set(e.pointerId, e)

  if (activePointers.size === 1) {
    isDragging = true
    startX = e.clientX
    startY = e.clientY
    startTranslateX = translateX.value
    startTranslateY = translateY.value
  } else if (activePointers.size === 2) {
    isDragging = false
    const pointers = Array.from(activePointers.values())
    const dx = pointers[0].clientX - pointers[1].clientX
    const dy = pointers[0].clientY - pointers[1].clientY
    initialPinchDistance = Math.sqrt(dx * dx + dy * dy)
    initialScale = scale.value
  }
}

function handlePointerMove(e: PointerEvent) {
  if (props.disableEventCapture || rotation.value % 360 !== 0) return
  if (!activePointers.has(e.pointerId)) return
  activePointers.set(e.pointerId, e)

  if (activePointers.size === 1 && isDragging) {
    const dx = e.clientX - startX
    const dy = e.clientY - startY
    translateX.value = startTranslateX + dx
    translateY.value = startTranslateY + dy
    clampTranslations()

    if (scale.value > 1) {
      setTransforming(true)
      startTransformTimer()
    }
  } else if (activePointers.size === 2) {
    const pointers = Array.from(activePointers.values())
    const dx = pointers[0].clientX - pointers[1].clientX
    const dy = pointers[0].clientY - pointers[1].clientY
    const distance = Math.sqrt(dx * dx + dy * dy)
    if (initialPinchDistance > 0) {
      const factor = distance / initialPinchDistance
      const newScale = initialScale * factor
      const currentMidX = (pointers[0].clientX + pointers[1].clientX) / 2
      const currentMidY = (pointers[0].clientY + pointers[1].clientY) / 2
      zoomToPoint(currentMidX, currentMidY, newScale)
    }
  }
}

function handlePointerUp(e: PointerEvent) {
  if (props.disableEventCapture || rotation.value % 360 !== 0) return
  activePointers.delete(e.pointerId)
  try {
    const target = e.currentTarget as HTMLElement
    target.releasePointerCapture(e.pointerId)
  } catch {
    // Ignore
  }

  if (activePointers.size === 0) {
    isDragging = false
    if (scale.value === 1) {
      setTransforming(false)
    } else {
      startTransformTimer()
    }
  } else if (activePointers.size === 1) {
    // Resume dragging with the remaining pointer
    const remaining = Array.from(activePointers.values())[0]
    isDragging = true
    startX = remaining.clientX
    startY = remaining.clientY
    startTranslateX = translateX.value
    startTranslateY = translateY.value
  }
}

function handlePointerCancel(e: PointerEvent) {
  if (props.disableEventCapture || rotation.value % 360 !== 0) return
  handlePointerUp(e)
}

function handleWheel(e: WheelEvent) {
  if (props.disableEventCapture || rotation.value % 360 !== 0) return
  const target = e.target as HTMLElement
  // Only zoom if the event isn't targeted inside overlays, info panels or menus
  if (
    target.closest('.media-info-panel') ||
    target.closest('.v-overlay') ||
    target.closest('.v-menu')
  ) {
    return
  }

  e.preventDefault()

  // Flag active transform state
  setTransforming(true)

  // Adjust zoom sensitivity based on device
  const zoomFactor = e.ctrlKey ? 0.05 : 0.01
  const direction = e.deltaY < 0 ? 1 : -1
  const newScale = scale.value * (1 + direction * zoomFactor * 10)
  zoomToPoint(e.clientX, e.clientY, newScale)

  if (scale.value > 1 && scale.value < 8) {
    startTransformTimer()
  }
}

useEventListener(containerRef, 'wheel', handleWheel, { passive: false })
</script>

<template>
  <div class="photo-viewer-wrapper">
    <div
      v-if="settings.useImageGlow && !is3DMode"
      class="blurry-bg"
      :style="{
        backgroundImage: `url(${imageUrl})`,
        transform: rotation ? `rotate(${rotation}deg)` : undefined,
      }"
    ></div>

    <!-- 3D mode: Instantiated when toggled to true -->
    <template v-if="is3DMode && panoramaConfig">
      <PanoramaViewer
        :config="panoramaConfig"
        :base-url="`${baseUrl}/hosted/pano/${mediaItemId}`"
      />
    </template>

    <div
      v-else
      ref="containerRef"
      class="zoom-pan-container"
      :class="{ 'zoomed-in': scale > 1 }"
      @pointerdown="handlePointerDown"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerUp"
      @pointercancel="handlePointerCancel"
    >
      <div class="image-wrapper" :style="wrapperStyle">
        <!-- Thumbnail layer at bottom -->
        <img
          ref="thumbRef"
          class="image-tag thumbnail-img"
          :src="imageUrl"
          v-if="!thumbErrored || !fullResLoaded"
          @error="thumbErrored = true"
          @load="clampTranslations"
          @dragstart.prevent
        />

        <!-- Full-resolution layer stacked on top -->
        <img
          v-if="fullResUrl"
          class="image-tag full-res-img"
          :src="fullResUrl"
          alt="Full resolution image"
          decoding="async"
          loading="lazy"
          :style="{
            opacity: fullResLoaded ? 1 : 0,
            pointerEvents: fullResLoaded ? 'auto' : 'none',
          }"
          @load="onFullResLoad"
          @dragstart.prevent
        />

        <!-- Motion Photo Video Overlay -->
        <video
          ref="videoPlayerRef"
          v-if="showingMotionVideo"
          class="image-tag motion-video-tag"
          :src="motionVideoUrl"
          autoplay
          muted
          playsinline
          @play="startMonitoring"
          @pause="stopMonitoring"
          @ended="onVideoEnded"
        />
      </div>
    </div>

    <!-- Bottom Center Panorama Toggle Control -->
    <div
      v-if="isPanorama && panoramaConfig"
      class="pano-toggle-container"
      :class="{ 'hide-ui': !showUi }"
    >
      <v-btn rounded="xl" variant="plain" class="pano-toggle-btn" @click="is3DMode = !is3DMode">
        <v-icon start :icon="is3DMode ? 'mdi-image-outline' : 'mdi-panorama-variant-outline'" />
        <span>{{ is3DMode ? 'View as Photo' : 'Enter 3D Panorama' }}</span>
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
.photo-viewer-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.blurry-bg {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-size: contain;
  background-position: center;
  filter: blur(50px) brightness(60%);
}

.zoom-pan-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  touch-action: none;
  cursor: default;
}

.zoom-pan-container.zoomed-in {
  cursor: grab;
}

.zoom-pan-container.zoomed-in:active {
  cursor: grabbing;
}

.image-wrapper {
  position: absolute;
  will-change: transform;
}

.image-tag {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  user-select: none;
}

.full-res-img {
  z-index: 2;
}

.motion-video-tag {
  z-index: 10;
  background-color: transparent;
}

/* Bottom Center Panorama Overlay controls to match ViewPhoto's top header bars */
.pano-toggle-container {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  padding: 8px 30px;
  border-radius: 30px;
  background-color: rgba(var(--bg, 0, 0, 0), 0.8);
  box-shadow: 0 3px 12px rgba(var(--bg, 0, 0, 0), 0.15);
  transition:
    background-color 0.15s,
    transform 0.5s ease-in-out;
  z-index: 1600;
  pointer-events: auto;
}

.pano-toggle-container.hide-ui {
  transform: translate(-50%, 80px);
}

.backdrop-blur .pano-toggle-container {
  backdrop-filter: blur(30px) saturate(150%) brightness(90%) contrast(90%);
  background-color: rgba(var(--bg, 0, 0, 0), 0.5);
  border: 1px solid rgba(var(--fg, 255, 255, 255), 0.1);
}

.backdrop-blur .pano-toggle-container:hover {
  background-color: rgba(var(--bg, 0, 0, 0), 0.7);
}

.pano-toggle-btn {
  text-transform: none;
  font-family: Jost, sans-serif;
  font-weight: 500;
  letter-spacing: 0.5px;
  color: rgba(var(--fg, 255, 255, 255), 0.95);
}
</style>
