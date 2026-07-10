<script setup lang="ts">
import MediaViewer from '@/vues/components/viewer/MediaViewer.vue'
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import { useUiHider } from '@/scripts/composables/useUiHider.ts'

const route = useRoute()

const isVideo = computed(() => route.params.isVideo === 'v')

const id = computed(() => {
  const rawId = route.params.mediaId
  if (rawId && !Array.isArray(rawId)) return rawId
  return null
})

const { showUI } = useUiHider(5)
</script>

<template>
  <main-layout-container>
    <media-viewer
      :class="{ 'hide-ui': !showUI }"
      :show-ui="showUI"
      class="viewer"
      v-if="id"
      :disable-event-capture="false"
      :is-video="isVideo"
      :media-item-id="id"
      :muted="false"
    />
    <div v-else>Incorrect URL</div>
  </main-layout-container>
</template>

<style scoped>
.viewer {
  width: 100%;
  height: 100%;
}

.hide-ui {
  cursor: none !important;
}

.hide-ui,
.hide-ui :deep(*) {
  cursor: none !important;
}
</style>
