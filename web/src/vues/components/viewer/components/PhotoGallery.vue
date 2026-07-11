<script setup lang="ts">
import { computed, nextTick, useTemplateRef, watch, ref } from 'vue'
import { useVirtualizer, type VirtualItem } from '@tanstack/vue-virtual'
import { useTimeoutFn } from '@vueuse/core'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

const props = defineProps<{
  ratio: number
  focusId: string
  queue: string[]
  height: number
  resizing?: boolean
}>()
const emit = defineEmits(['change-focus'])

const scrollContainerEl = useTemplateRef<HTMLDivElement>('scrollContainer')
const NON_FOCUS_RATIO = 0.5
const PADDING = 5
const GAP = 4
const FOCUS_MARGIN = 10

const activeFocusId = ref(props.focusId)

const virtualizerOptions = computed(() => ({
  count: props.queue.length,
  getScrollElement: () => scrollContainerEl.value,
  estimateSize: (index: number) => {
    const id = props.queue[index]
    const isFocused = id === activeFocusId.value
    const baseSize = isFocused
      ? (props.height - PADDING * 2) * props.ratio + FOCUS_MARGIN * 2
      : props.height * NON_FOCUS_RATIO
    return baseSize + GAP
  },
  horizontal: true,
  overscan: 10,
}))

const virtualizer = useVirtualizer(virtualizerOptions)

// Dynamically calculate individual dimensions and absolute positioning shifts
function getThumbStyle(virtualItem: VirtualItem) {
  const isFocused = props.queue[virtualItem.index] === activeFocusId.value
  const width = isFocused ? virtualItem.size - FOCUS_MARGIN * 2 - GAP : virtualItem.size - GAP
  const translateX = isFocused ? virtualItem.start + FOCUS_MARGIN : virtualItem.start

  return {
    width: `${width}px`,
    height: `${props.height - PADDING * 2}px`,
    transform: `translateX(${translateX}px)`,
  }
}

let lastFocusIdChangeTime = 0
let isRapid = false
const RAPID_NAVIGATION_THRESHOLD = 180 // ms

const { start: startDefer, stop: stopDefer } = useTimeoutFn(
  (newId: string) => {
    activeFocusId.value = newId
  },
  100,
  { immediate: false },
)

// 1. Listen for raw prop focus changes to gauge transition speeds and queue delays
watch(
  () => props.focusId,
  (newId) => {
    if (!newId) return
    stopDefer()

    const now = performance.now()
    const isInitial = lastFocusIdChangeTime === 0
    isRapid = isInitial || now - lastFocusIdChangeTime < RAPID_NAVIGATION_THRESHOLD
    lastFocusIdChangeTime = now

    if (isRapid) {
      // Rapid skipping / loop: don't delay layout updates so it snaps instantly
      activeFocusId.value = newId
    } else {
      // Delay setting new focus id for 100ms to avoid lag spike during animation
      startDefer(newId)
    }
  },
  { immediate: true },
)

// 2. Watch active ID, ratio, and height updates to trigger scroll centering and remeasuring
watch(
  [activeFocusId, () => props.ratio, () => props.height],
  ([newId]) => {
    if (!newId) return
    virtualizer.value.measure()
    const index = props.queue.indexOf(newId)
    if (index === -1) return
    const useInstant = isRapid || props.resizing
    nextTick(() => {
      virtualizer.value.scrollToIndex(index, {
        align: 'center',
        behavior: useInstant ? 'auto' : 'smooth',
      })
    })
  },
  { immediate: true },
)

// Watch for queue updates to ensure layout remeasures
watch(
  () => props.queue,
  () => {
    virtualizer.value.measure()
  },
  { deep: true },
)
</script>

<template>
  <div ref="scrollContainer" class="gallery-container" :class="{ resizing: props.resizing }">
    <div
      class="gallery-inner"
      :style="{
        width: `${virtualizer.getTotalSize()}px`,
      }"
    >
      <thumbnail-img
        @click="emit('change-focus', queue[virtualItem.index]!)"
        v-for="virtualItem in virtualizer.getVirtualItems()"
        :key="virtualItem.key"
        :media-item-id="queue[virtualItem.index]!"
        :height="144"
        cover
        loading="lazy"
        decoding="async"
        class="gallery-thumb"
        :style="getThumbStyle(virtualItem)"
      />
    </div>
  </div>
</template>

<style scoped>
.gallery-container {
  width: 100%;
  height: 100%;
  overflow-x: auto;
  overflow-y: hidden;
  position: relative;
  scrollbar-width: none;
  padding: calc(v-bind(PADDING) * 1px);
  box-sizing: border-box;
}

.gallery-container::-webkit-scrollbar {
  display: none;
}

.gallery-inner {
  height: 100%;
  position: relative;
  box-sizing: border-box;
}

.gallery-thumb {
  cursor: pointer;
  position: absolute;
  top: 0;
  left: 0;
  overflow: hidden;
  box-sizing: border-box;
  border-radius: calc((v-bind(height) * 1px) / 13);

  /* Transition widths and absolute shifts smoothly on standard navigation */
  transition:
    width 0.5s cubic-bezier(0.25, 0.8, 0.25, 1),
    transform 0.5s cubic-bezier(0.25, 0.8, 0.25, 1);
  will-change: transform;
}

/* Disable transitions entirely on the thumbnails while resizing is in progress */
.gallery-container.resizing .gallery-thumb {
  transition: none !important;
}
</style>
