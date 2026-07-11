<script setup lang="ts">
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

defineProps<{
  ratio: number
  focusId: string
  queue: string[]
}>()
</script>

<template>
  <div>
    <div class="gallery">
      <div
        v-for="id in queue"
        :key="id"
        class="gallery-img"
        :style="{
          height: `96px`,
          width: `${focusId === id ? 96 * ratio : 50}px`,
        }"
      >
        <thumbnail-img
          :media-item-id="id"
          :height="144"
          cover
          loading="lazy"
          decoding="async"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.gallery {
  width: 100%;
  height: 100%;
  position: relative;
  background-color: red;
  display: flex;
  gap: 2px;
  padding: 2px;
}

.gallery-img {
}
</style>
