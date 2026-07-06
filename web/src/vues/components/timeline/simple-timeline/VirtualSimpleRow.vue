<script setup lang="ts">
import type { SimpleLayoutRow } from '@/scripts/types/timeline/layout.ts'
import GridItem from '@/vues/components/timeline/timeline-components/GridItem.vue'
import AsyncGridItem from '@/vues/components/timeline/timeline-components/AsyncGridItem.vue'

defineProps<{
  item: SimpleLayoutRow
  containerWidth: number
  itemGap: number
  isScrollingFast: boolean
  asyncDecoding: boolean
  viewLink: string
}>()
</script>

<template>
  <div>
    <div
      class="virtual-scroll-row"
      :style="{
        height: `${Math.round(item.height)}px`,
        width: `${containerWidth}px`,
        marginBottom: `${itemGap}px`,
      }"
    >
      <template v-if="asyncDecoding">
        <async-grid-item
          v-for="mediaItem in item.items"
          :key="mediaItem.id"
          :width="Math.round(mediaItem.ratio * item.height)"
          :height="Math.round(item.height)"
          :thumbnail-size="item.thumbnailSize"
          :media-item="mediaItem"
          :is-scrolling-fast="isScrollingFast"
          :view-link="viewLink"
        />
      </template>
      <template v-else>
        <grid-item
          v-for="mediaItem in item.items"
          :key="mediaItem.id"
          :width="Math.round(mediaItem.ratio * item.height)"
          :height="Math.round(item.height)"
          :thumbnail-size="item.thumbnailSize"
          :media-item="mediaItem"
          :is-scrolling-fast="isScrollingFast"
          :view-link="viewLink"
        />
      </template>
    </div>
  </div>
</template>

<style scoped>
.virtual-scroll-row {
  display: flex;
  gap: var(--item-gap);
  overflow: hidden;
  contain: paint;
}
</style>
