<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'

const props = defineProps<{
  mediaItemIds: string[]
}>()

const truncatedIds = computed(() => props.mediaItemIds.slice(0, 5))

const thumbCache = ref(new Map<string, boolean>())
function tryThumb(thumbId: string | null) {
  if (thumbId == null) return null
  const img = new Image()
  img.src = mediaItemService.getPhotoThumbnail(thumbId, 144, false)
  img.onload = () => thumbCache.value.set(thumbId, false)
  img.onerror = () => thumbCache.value.set(thumbId, true)
}

watch(truncatedIds, () => truncatedIds.value.forEach((id) => tryThumb(id)), { immediate: true })
</script>

<template>
  <div class="album-preview">
    <div
      class="image-stack"
      :style="{
        marginTop: `${truncatedIds.length === 1 ? 15 : truncatedIds.length === 2 ? 30 : 35}px`,
      }"
    >
      <div
        v-for="(id, i) in truncatedIds"
        :key="id"
        class="stacked-img"
        :style="{
          '--percentage': (truncatedIds.length - i) * (1 / truncatedIds.length),
          '--i': i,
          backgroundImage: `url(${mediaItemService.getPhotoThumbnail(id, 144, thumbCache.get(id) ?? false)})`,
        }"
      />
    </div>

    <p class="items-text">
      <b>{{ mediaItemIds.length }}</b> item{{ mediaItemIds.length === 1 ? '' : 's' }}
    </p>
  </div>
</template>

<style scoped>
.album-preview {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.image-stack {
  margin-top: 40px;
  margin-bottom: 30px;
  width: 90px;
  height: 90px;
  position: relative;
}

.stacked-img {
  position: absolute;
  top: 0;
  left: 0;

  width: 100%;
  height: 100%;

  border-radius: 8px;
  object-fit: cover;
  background-size: cover;
  background-position: center;
  box-shadow: 0 -3px 8px 0 rgba(0, 0, 0, 0.2);

  /* Stack offset per image */
  z-index: calc(100 - var(--i));
  transform: translateY(calc((1 - var(--percentage)) * -50px))
    scale(calc(1 + var(--percentage) / 3));
  opacity: calc(pow(var(--percentage), 2));
}
</style>
