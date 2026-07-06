<!-- File: src/vues/components/viewer/pano/PanoramaViewer.vue -->
<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import type { PannellumConfig } from '@/scripts/types/api/pannellumConfig.js'
import 'pannellum'
import 'pannellum/build/pannellum.css'

interface PannellumViewer {
  destroy: () => void
}

declare global {
  interface Window {
    pannellum?: {
      viewer: (container: HTMLDivElement, config: PannellumConfig) => PannellumViewer
    }
  }
}

const props = defineProps<{
  baseUrl: string
  config: PannellumConfig
}>()

const containerRef = ref<HTMLDivElement | null>(null)
let viewerInstance: PannellumViewer | null = null

function destroyViewer() {
  if (viewerInstance) {
    try {
      viewerInstance.destroy()
    } catch (e) {
      console.warn('Error destroying Pannellum instance:', e)
    }
    viewerInstance = null
  }
}

function initViewer() {
  destroyViewer()
  if (!containerRef.value) return

  // Ensure DOM transitions and sizes are computed
  nextTick(() => {
    if (!containerRef.value) return
    try {
      // Access via global window or fallback import reference
      const pannellumLib = window.pannellum
      if (pannellumLib) {
        const config = props.config
        if (config.multiRes) {
          config.multiRes.basePath = props.baseUrl
        }
        config.autoLoad = true
        config.showZoomCtrl = false
        config.compass = true
        config.showFullscreenCtrl = false
        config.avoidShowingBackground = false
        config.friction = 0.1
        config.autoRotate = -10
        config.autoRotateStopDelay = 400
        viewerInstance = pannellumLib.viewer(containerRef.value, config)
      } else {
        console.error('Pannellum object is not registered globally.')
      }
    } catch (err) {
      console.error('Failed to initialize Pannellum engine:', err)
    }
  })
}

onMounted(() => {
  initViewer()
})

onBeforeUnmount(() => {
  destroyViewer()
})

// Reinitialize when config changes
watch(
  () => props.config,
  () => {
    initViewer()
  },
  { deep: true },
)
</script>

<template>
  <div ref="containerRef" class="pannellum-viewer-container" />
</template>

<style scoped>
.pannellum-viewer-container {
  width: 100%;
  height: 100%;
  background-color: #000;
  position: relative;
}

/* Ensure deep styling applies to Pannellum wrapper elements */
:deep(.pnlm-container) {
  background-color: #000 !important;
  width: 100% !important;
  height: 100% !important;
}

:deep(.pnlm-load-button) {
  background-color: rgba(var(--v-theme-primary), 0.8) !important;
  color: white !important;
  border-radius: 20px !important;
}
</style>
