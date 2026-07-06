<script setup lang="ts">
import { computed } from 'vue'
import type { StorageReviewItem } from '@/scripts/types/generated/timeline.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { prettyBytes, toHms } from '@/scripts/utils.ts'

const props = defineProps<{
  item: StorageReviewItem
  basePath: string
  tileWidth: number
  isSelected: boolean
  isSelecting: boolean // Selection mode indicator
  isDownloading: boolean
  actionLoading: boolean
  batchDownloading: boolean
}>()

const emit = defineEmits(['toggle', 'download', 'delete'])

const formattedSize = computed(() => prettyBytes(props.item.sizeBytes))
const formattedDuration = computed(() => toHms((props.item.durationMs ?? 0) / 1000))

const formattedDate = computed(() => {
  const date = new Date(props.item.takenAtLocal)
  if (Number.isNaN(date.getTime())) return 'Unknown date'
  return date.toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
})

const roundedScore = computed(() =>
  props.item.weightedScore !== undefined ? Math.round(props.item.weightedScore) : null,
)
</script>

<template>
  <article
    class="review-item"
    :style="{ width: `${tileWidth}px` }"
    :class="{ 'is-selected': isSelected }"
  >
    <div class="thumb-container">
      <!-- Option A: Selection mode is active. Clicking preview toggles selection -->
      <div v-if="isSelecting" class="thumb-link select-trigger" @click="emit('toggle')">
        <img
          decoding="async"
          loading="lazy"
          :src="mediaItemService.getPhotoThumbnail(item.id, 240, !item.hasThumbnails)"
          class="thumbnail-img"
        />
        <div class="video-chip" v-if="item.isVideo">
          <v-icon icon="mdi-play" size="16" />
          <span>{{ formattedDuration }}</span>
        </div>
      </div>

      <!-- Option B: Selection mode is NOT active. Clicking preview navigates to full view -->
      <router-link v-else class="thumb-link" :to="`${basePath}/view/${item.id}`">
        <img
          decoding="async"
          loading="lazy"
          :src="mediaItemService.getPhotoThumbnail(item.id, 240, !item.hasThumbnails)"
          class="thumbnail-img"
        />
        <div class="video-chip" v-if="item.isVideo">
          <v-icon icon="mdi-play" size="16" />
          <span>{{ formattedDuration }}</span>
        </div>
      </router-link>

      <!-- Floating Fullscreen Shortcut: Shown in selection mode during hover -->
      <router-link
        v-if="isSelecting"
        class="fullscreen-btn"
        :to="`${basePath}/view/${item.id}`"
        title="View in fullscreen"
        @click.stop
      >
        <v-icon size="18" icon="mdi-fullscreen" />
      </router-link>

      <!-- Selector button circle -->
      <button
        type="button"
        class="select-overlay-btn"
        :class="{ visible: isSelecting }"
        @click.stop="emit('toggle')"
      >
        <v-icon
          :icon="isSelected ? 'mdi-checkbox-marked-circle' : 'mdi-checkbox-blank-circle-outline'"
          size="20"
        />
      </button>
    </div>

    <div class="item-info">
      <div class="meta">
        <strong>{{ formattedSize }}</strong>
        <span>{{ item.filename }}</span>
        <span>{{ formattedDate }}</span>
        <span v-if="roundedScore !== null"> Quality {{ roundedScore }} </span>
      </div>
      <div class="item-actions">
        <button
          type="button"
          class="action-btn download"
          :disabled="isDownloading || batchDownloading"
          title="Download original"
          @click="emit('download')"
        >
          <v-icon v-if="!isDownloading" icon="mdi-download" color="primary" size="20" />
          <v-progress-circular v-else indeterminate color="primary" size="16" width="2" />
        </button>
        <button
          type="button"
          class="action-btn delete"
          :disabled="isDownloading || actionLoading || batchDownloading"
          title="Move to bin"
          @click="emit('delete')"
        >
          <v-icon icon="mdi-delete" color="error" size="20" />
        </button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.review-item {
  height: 100%;
  overflow: hidden;
  border-radius: 28px;
  background: rgb(var(--v-theme-surface-container));
  transition:
    background-color 0.16s ease,
    box-shadow 0.16s ease;
}

.thumb-container {
  position: relative;
  height: 210px;
  overflow: hidden;
  border-radius: 28px;
}

.thumb-link {
  display: block;
  height: calc(100% - 16px);
  width: calc(100% - 16px);
  margin: 8px;
  border-radius: 20px;
  overflow: hidden;
  background: rgba(var(--v-theme-on-surface), 0.05);
  transition:
    transform 0.15s ease-in-out,
    box-shadow 0.15s ease-in-out;
}

.thumb-link.select-trigger {
  cursor: pointer;
}

/* Clear visual highlight and shrink response to match GridItem.vue */
.is-selected .thumb-link {
  box-shadow:
    inset 0 0 0 2px rgb(var(--v-theme-primary)),
    0 0 0 4px rgba(var(--v-theme-primary), 0.4);
  transform: scale(0.96);
  border-radius: 20px;
}

.thumbnail-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

/* Hidden by default; displayed on hover or when selection processes are active */
.select-overlay-btn {
  position: absolute;
  top: 10px;
  left: 10px;
  z-index: 2;
  color: rgba(255, 255, 255, 0.85);
  background: rgba(0, 0, 0, 0.35);
  border: none;
  border-radius: 50%;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 0.15s ease,
    background-color 0.15s ease,
    color 0.15s ease,
    transform 0.15s ease;
}

.review-item:hover .select-overlay-btn,
.is-selected .select-overlay-btn,
.select-overlay-btn.visible {
  opacity: 1;
  pointer-events: auto;
}

.select-overlay-btn:hover {
  background: rgba(0, 0, 0, 0.6);
  transform: scale(1.05);
}

.is-selected .select-overlay-btn {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
}

/* floating fullscreen shortcut icon style */
.fullscreen-btn {
  position: absolute;
  bottom: 10px;
  left: 10px;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.6);
  color: white;
  display: none;
  align-items: center;
  justify-content: center;
  text-decoration: none;
  z-index: 2;
  transition: transform 0.15s ease-in-out;
}

.fullscreen-btn:hover {
  transform: scale(1.1);
  background: rgba(0, 0, 0, 0.8);
}

.review-item:hover .fullscreen-btn {
  display: flex;
}

.video-chip {
  position: absolute;
  top: 10px;
  right: 10px;
  display: flex;
  align-items: center;
  gap: 3px;
  color: white;
  background: rgba(0, 0, 0, 0.62);
  border-radius: 999px;
  padding: 4px 9px;
  font-size: 0.78rem;
}

.item-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
}

.meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.meta strong {
  font-size: 1rem;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.meta span {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.8rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-actions {
  display: flex;
  flex-shrink: 0;
  gap: 6px;
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: none;
  background: transparent;
  cursor: pointer;
  transition:
    background-color 0.15s ease,
    color 0.15s ease;
}

.action-btn:hover:not(:disabled) {
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.action-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
