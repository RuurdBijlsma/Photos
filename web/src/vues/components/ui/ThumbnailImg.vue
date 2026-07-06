<script setup lang="ts">
import { ref } from 'vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'

withDefaults(
  defineProps<{
    mediaItemId: string
    height?: number
    width?: number
    cover?: boolean
    decoding?: 'async' | 'auto' | 'sync'
    loading?: 'eager' | 'lazy'
  }>(),
  {},
)

const useOnDemandThumb = ref(false)
</script>

<template>
  <img
    :decoding="decoding"
    :loading="loading"
    :height="height"
    :width="width"
    :src="mediaItemService.getPhotoThumbnail(mediaItemId, height ?? 480, useOnDemandThumb)"
    @error="useOnDemandThumb = true"
    :class="{
      'cover-img': cover,
      'contain-img': !cover,
    }"
  />
</template>

<style scoped>
.cover-img {
  object-fit: cover;
  height: 100%;
}

.contain-img {
  object-fit: contain;
}
</style>
