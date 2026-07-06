<script setup lang="ts">
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import { toHms } from '@/scripts/utils.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { computed } from 'vue'

const props = defineProps<{
  mediaItem: SimpleTimelineItem
  width: number
  height: number
  thumbnailSize: number
}>()

const emit = defineEmits(['drag-start'])

const id = computed(() => props.mediaItem.id)
const isVideo = computed(() => props.mediaItem.isVideo)
const durationMs = computed(() => props.mediaItem.durationMs)

function onDragStart(e: DragEvent) {
  if (e.dataTransfer) {
    e.dataTransfer.setData('text/plain', id.value)
    e.dataTransfer.effectAllowed = 'move'
  }
  emit('drag-start', id.value)
}
</script>

<template>
  <div
    class="reorder-grid-item"
    draggable="true"
    @dragstart="onDragStart"
    :style="{
      width: `${width}px`,
      height: `${height}px`,
    }"
  >
    <div class="item-content">
      <img
        :src="mediaItemService.getPhotoThumbnail(id, thumbnailSize, !mediaItem.hasThumbnails)"
        :alt="id"
        draggable="false"
      />
      <div class="video-info" v-if="isVideo">
        <span v-if="durationMs">{{ toHms(durationMs / 1000) }}</span>
        <v-icon color="white" size="16" icon="mdi-play" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.reorder-grid-item {
  flex: 0 0 auto;
  cursor: grab;
  user-select: none;
}

.reorder-grid-item:active {
  cursor: grabbing;
}

.item-content {
  width: 100%;
  height: 100%;
  position: relative;
  border-radius: 12px;
  overflow: hidden;
  transform: scale(0.9);
  transition: transform 0.2s ease;
  background-color: rgba(var(--v-theme-on-surface), 0.05);
}

.reorder-grid-item:hover .item-content {
  transform: scale(0.92);
}

img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  pointer-events: none;
}

.video-info {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 4px;
  background: rgba(0, 0, 0, 0.6);
  padding: 2px 6px;
  border-radius: 4px;
}

.video-info span {
  font-size: 12px;
  font-weight: 600;
  color: white;
}
</style>
