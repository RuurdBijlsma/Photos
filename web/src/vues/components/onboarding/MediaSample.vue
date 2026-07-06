<script setup lang="ts">
import { computed } from 'vue'
import type { MediaSampleResponse } from '@/scripts/types/api/admin.js'

const props = defineProps<{
  mediaSamples: MediaSampleResponse | null
  images: { imageUrl: string; relPath: string }[]
}>()

const totalMediaCount = computed(() => {
  if (!props.mediaSamples) return 0
  return props.mediaSamples.photoCount + props.mediaSamples.videoCount
})

const cardTitle = computed(() => {
  return props.mediaSamples
    ? `Media Files (${totalMediaCount.value.toLocaleString()} Supported Files)`
    : 'Media Files'
})

const cardCaption = computed(() => {
  if (!props.mediaSamples) {
    return "We're scanning your library for media samples to display."
  }
  const { photoCount, videoCount } = props.mediaSamples
  const photoText = `${photoCount.toLocaleString()} photos`
  const videoText = `${videoCount.toLocaleString()} videos`

  let caption = `There's ${photoText} and ${videoText} in your library.`
  if (photoCount > 0) {
    caption += ' Here’s a sample:'
  }
  return caption
})
</script>

<template>
  <v-card rounded class="folder-card" variant="text">
    <v-card-title class="d-flex align-center">
      <v-icon icon="mdi-eye-check-outline" class="mr-2"></v-icon>
      {{ cardTitle }}
    </v-card-title>

    <v-card-text>
      <p class="text-caption text-medium-emphasis mb-4">
        {{ cardCaption }}
      </p>

      <div class="image-grid">
        <div v-for="(image, index) in images" :key="index">
          <div v-if="!image.imageUrl" class="preview-skeleton">
            <v-progress-circular :size="130" indeterminate color="surface-container" />
          </div>

          <a
            v-else
            v-ripple
            :href="image.imageUrl"
            target="_blank"
            v-tooltip:top="image.relPath"
            class="preview-image"
            :style="{ backgroundImage: `url(${image.imageUrl})` }"
            aria-label="View image preview"
          >
          </a>
        </div>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.image-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 15px;
  margin-top: 10px;
}

.preview-skeleton,
.preview-image {
  --border-radius-transition: border-radius 0.2s ease-in-out;
  width: 100%;
  aspect-ratio: 1;
  border-radius: 50%;
  background-color: rgb(var(--v-theme-surface-container));
  transition: var(--border-radius-transition);
  will-change: border-radius;
}

.preview-skeleton {
  cursor: progress;
  display: flex;
  justify-content: center;
  align-items: center;
}

.preview-image {
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  cursor: pointer;
  display: block;
}

.preview-skeleton:hover,
.preview-image:hover {
  border-radius: 15px;
}
</style>
