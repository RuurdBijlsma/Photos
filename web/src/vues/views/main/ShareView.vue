<script setup lang="ts">
import MediaViewer from '@/vues/components/viewer/MediaViewer.vue'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import { useUiHider } from '@/scripts/composables/useUiHider.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.ts'

const route = useRoute()
const mediaItemStore = useMediaItemStore()
const authStore = useAuthStore()

const isVideo = computed(() => route.params.viewerType === 'v')
const isPano = computed(() => route.params.viewerType === 'pano')

const id = computed(() => {
  const rawId = route.params.mediaId
  if (rawId && !Array.isArray(rawId)) return rawId
  return null
})

const panoConfig = ref<PannellumConfig | undefined>(undefined)

watch(
  id,
  () => {
    if (!id.value) return
    if (authStore.isAuthenticated) {
      mediaItemStore.fetchMediaItem(id.value)
    } else if (isPano.value) {
      const changeId = id.value
      mediaItemService.getPanoConfig(changeId).then((d) => {
        if (id.value === changeId) {
          panoConfig.value = d.data
        }
      })
    }
  },
  { immediate: true },
)

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
      :elemental-fullscreen="true"
      :force-pano="panoConfig"
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
