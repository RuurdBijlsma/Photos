<script setup lang="ts">
import MdiChevronLeft from '~icons/mdi/chevron-left'
import MdiChevronRight from '~icons/mdi/chevron-right'
import { ref } from 'vue'
import MediaViewer from '@/vues/components/viewer/MediaViewer.vue'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

withDefaults(
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

const showLeftButton = ref(false)
const showRightButton = ref(false)
const isZoomed = ref(false)
const isPanoActive = ref(false)

function onZoomChange(zoomed: boolean) {
  isZoomed.value = zoomed
  emit('zoom-change', zoomed)
}

function onPanoActive(active: boolean) {
  isPanoActive.value = active
  emit('pano-active', active)
}

function forwardWheel(e: WheelEvent) {
  const zoomPanContainer = document.querySelector('.zoom-pan-container')
  if (!zoomPanContainer) return

  e.preventDefault()

  const forwardedEvent = new WheelEvent('wheel', {
    clientX: e.clientX,
    clientY: e.clientY,
    deltaX: e.deltaX,
    deltaY: e.deltaY,
    deltaMode: e.deltaMode,
    ctrlKey: e.ctrlKey,
    metaKey: e.metaKey,
    shiftKey: e.shiftKey,
    altKey: e.altKey,
    bubbles: true,
    cancelable: true,
  })

  zoomPanContainer.dispatchEvent(forwardedEvent)
}
</script>

<template>
  <div
    class="desktop-media-viewer"
    :style="{
      height: `calc(100% - ${showGallery ? photoGalleryHeight : 0}px)`,
    }"
  >
    <MediaViewer
      :disable-event-capture="disableEventCapture"
      :muted="muted"
      :is-video="isVideo"
      :media-item-id="mediaItemId"
      :show-ui="showUi"
      :elemental-fullscreen="elementalFullscreen"
      :force-pano="forcePano"
      :active="true"
      class="photo-viewer"
      @zoom-change="onZoomChange"
      @pano-active="onPanoActive"
      @buffering="emit('buffering', $event)"
    />

    <!-- Left Chevron Hover Zone -->
    <div
      v-if="prevId !== null"
      class="prev-area"
      :class="{ 'zoomed-nav': isZoomed || isPanoActive }"
      @click="emit('prev')"
      @mouseenter="showLeftButton = true"
      @mouseleave="showLeftButton = false"
      @wheel="forwardWheel"
    >
      <v-btn
        class="nav-btn"
        :class="{ 'show-btn': showLeftButton }"
        :icon="MdiChevronLeft"
        variant="elevated"
        rounded="xl"
        size="70"
      />
    </div>

    <!-- Right Chevron Hover Zone -->
    <div
      v-if="nextId !== null"
      class="next-area"
      :class="{ 'zoomed-nav': isZoomed || isPanoActive }"
      @click="emit('next')"
      @mouseenter="showRightButton = true"
      @mouseleave="showRightButton = false"
      @wheel="forwardWheel"
    >
      <v-btn
        class="nav-btn"
        :class="{ 'show-btn': showRightButton }"
        :icon="MdiChevronRight"
        variant="elevated"
        rounded="xl"
        size="70"
      />
    </div>
  </div>
</template>

<style scoped>
.desktop-media-viewer {
  width: 100%;
  position: absolute;
  top: 0;
  left: 0;
  z-index: 1500;
}

.photo-viewer {
  width: 100%;
  height: 100%;
  background-color: rgb(var(--bg));
}

.prev-area,
.next-area {
  position: absolute;
  top: 150px;
  height: calc(100% - 360px);
  width: 15%;
  min-width: 92px;
  cursor: pointer;
  display: flex;
  align-items: center;
  padding: 20px;
  z-index: 1502;
}

.prev-area {
  left: 0;
  justify-content: flex-start;
}

.next-area {
  right: 0;
  justify-content: flex-end;
}

.nav-btn {
  opacity: 0;
  transition: opacity 0.15s ease-in-out;
  pointer-events: auto;
}

.nav-btn.show-btn {
  opacity: 1;
}

.prev-area.zoomed-nav,
.next-area.zoomed-nav {
  pointer-events: none;
  cursor: default;
}

.prev-area.zoomed-nav .nav-btn:hover,
.next-area.zoomed-nav .nav-btn:hover {
  opacity: 1;
}
</style>
