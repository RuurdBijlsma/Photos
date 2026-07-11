<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

const PhotoViewer = defineAsyncComponent(
  () => import('@/vues/components/viewer/viewers/PhotoViewer.vue'),
)
const VideoViewer = defineAsyncComponent(
  () => import('@/vues/components/viewer/viewers/VideoViewer.vue'),
)

defineProps<{
  disableEventCapture: boolean
  isVideo: boolean
  mediaItemId: string
  muted: boolean
  showUi?: boolean
  autoplay?: boolean
  elementalFullscreen: boolean
  forcePano?: PannellumConfig | undefined
}>()

const emit = defineEmits<{
  (e: 'zoom-change', isZoomed: boolean): void
  (e: 'pano-active', isActive: boolean): void
}>()
</script>

<template>
  <div class="viewer-container">
    <photo-viewer
      :media-item-id="mediaItemId"
      v-if="!isVideo"
      :disable-event-capture="disableEventCapture"
      :show-ui="showUi"
      @zoom-change="emit('zoom-change', $event)"
      @pano-active="emit('pano-active', $event)"
      :force-pano="forcePano"
    />
    <video-viewer
      :media-item-id="mediaItemId"
      :elemental-fullscreen="elementalFullscreen"
      v-else
      :muted="muted"
      :show-ui="showUi"
      :autoplay="autoplay"
    />
  </div>
</template>

<style scoped>
.viewer-container {
  position: relative;
}
</style>
