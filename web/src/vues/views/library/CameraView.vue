<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, ref, useTemplateRef, watch } from 'vue'
import { useCameraStore } from '@/scripts/stores/cameraStore.ts'
// eslint-disable-next-line
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'

const route = useRoute()
const simpleTimeline = useTemplateRef('simpleTimeline')
const cameraStore = useCameraStore()

const isInitialLoad = ref(true)

const cameraMake = computed(() => {
  const param = route.params.cameraMake
  return Array.isArray(param) ? param[0] : param || ''
})

const cameraModel = computed(() => {
  const param = route.params.cameraModel
  return Array.isArray(param) ? param[0] : param || ''
})

const cameraId = computed(() => cameraMake.value + cameraModel.value)

const cameraResponse = computed(() => {
  if (!cameraMake.value || !cameraModel.value) return null
  return cameraStore.cameraMedia.get(cameraId.value) ?? null
})

const cameraInfo = computed(() => cameraResponse.value?.camera ?? null)
const items = computed(() => cameraResponse.value?.items ?? [])

async function loadCameraMedia() {
  if (!cameraMake.value || !cameraModel.value) return
  isInitialLoad.value = true
  try {
    await cameraStore.fetchCameraMedia(cameraMake.value, cameraModel.value)
  } finally {
    isInitialLoad.value = false
  }
}

watch(
  [cameraMake, cameraModel],
  () => {
    simpleTimeline.value?.scrollToTop()
    loadCameraMedia()
  },
  { immediate: true },
)
</script>

<template>
  <div class="camera-container">
    <simple-timeline
      ref="simpleTimeline"
      :timeline-items="items"
      :view-link="`/camera/${encodeURIComponent(cameraMake)}/${encodeURIComponent(cameraModel)}/view/`"
      v-if="cameraMake && cameraModel"
      :context="{}"
    >
      <div class="camera-header">
        <div class="camera-header-left">
          <div class="camera-icon-container">
            <v-icon icon="mdi-camera" size="64" color="primary" />
          </div>
        </div>
        <div class="camera-header-right">
          <h1 class="camera-title">
            {{ cameraInfo?.model || cameraModel || 'Unknown Model' }}
          </h1>
          <p class="camera-meta">
            <span class="camera-brand">{{
              cameraInfo?.make || cameraMake || 'Unknown Brand'
            }}</span>
            <span class="bullet">•</span>
            <span class="camera-count">
              {{ (cameraInfo?.photoCount ?? items.length).toLocaleString() }} item{{
                (cameraInfo?.photoCount ?? items.length) === 1 ? '' : 's'
              }}
            </span>
          </p>
        </div>
      </div>

      <div class="empty-camera" v-if="items.length === 0 && !isInitialLoad">
        <v-icon color="on-surface-variant" size="200" icon="mdi-camera-off"></v-icon>
        <h2>No media found for this camera</h2>
      </div>
    </simple-timeline>
  </div>
</template>

<style scoped>
.empty-camera {
  height: 700px;
  width: 100%;
  display: flex;
  place-items: center;
  place-content: center;
  flex-direction: column;
  color: rgb(var(--v-theme-on-surface-variant));
}

.camera-container {
  display: flex;
  flex-direction: column;
  width: 100%;
}

.camera-header {
  display: flex;
  align-items: center;
  width: 100%;
  margin-bottom: 20px;
}

.camera-header-left {
  padding: 10px;
}

.camera-icon-container {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 120px;
  height: 120px;
  border-radius: 30px;
  background-color: rgb(var(--v-theme-surface-container-high));
}

.camera-header-right {
  padding: 20px;
  flex-grow: 1;
}

.camera-title {
  margin: 0;
  font-weight: 500;
  font-size: 50px;
  line-height: 1.2;
  color: rgb(var(--v-theme-on-background));
  padding: 5px 3px;
  user-select: text;
}

.camera-meta {
  gap: 5px;
  font-weight: 400;
  font-size: 16px;
  opacity: 0.7;
  margin: 0;
  display: flex;
  align-items: center;
  padding-left: 3px;
}

.bullet {
  margin: 0 5px;
}
</style>
