<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

const PhotoViewer = defineAsyncComponent(
  () => import('@/vues/components/viewer/viewers/PhotoViewer.vue'),
)
const VideoViewer = defineAsyncComponent(
  () => import('@/vues/components/viewer/viewers/VideoViewer.vue'),
)

withDefaults(
  defineProps<{
    disableEventCapture: boolean
    isVideo: boolean
    mediaItemId: string
    muted: boolean
    showUi?: boolean
    autoplay?: boolean
    elementalFullscreen: boolean
    forcePano?: PannellumConfig | undefined
    active?: boolean
  }>(),
  {
    active: true,
  },
)

const emit = defineEmits<{
  (e: 'zoom-change', isZoomed: boolean): void
  (e: 'pano-active', isActive: boolean): void
  (e: 'buffering', isBuffering: boolean): void
}>()
</script>

<template>
  <div class="viewer-container">
    <photo-viewer
      :media-item-id="mediaItemId"
      v-if="!isVideo"
      :active="active"
      :disable-event-capture="disableEventCapture"
      :show-ui="showUi"
      @zoom-change="emit('zoom-change', $event)"
      @pano-active="emit('pano-active', $event)"
      :force-pano="forcePano"
    />
    <video-viewer
      :media-item-id="mediaItemId"
      :active="active"
      :elemental-fullscreen="elementalFullscreen"
      v-else
      :muted="muted"
      :show-ui="showUi"
      :autoplay="autoplay"
      @buffering="emit('buffering', $event)"
    />
  </div>
</template>

<style scoped>
.viewer-container {
  position: relative;
  width: 100%;
  height: 100%;
}
</style>
