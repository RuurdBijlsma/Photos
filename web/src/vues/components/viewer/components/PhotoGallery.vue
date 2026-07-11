<script setup lang="ts">
import { computed, nextTick, useTemplateRef, watch } from 'vue'
import { useVirtualizer, type VirtualItem } from '@tanstack/vue-virtual'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

const props = defineProps<{
  ratio: number
  focusId: string
  queue: string[]
  height: number
}>()

const scrollContainerEl = useTemplateRef<HTMLDivElement>('scrollContainer')
const NON_FOCUS_RATIO = 0.5
const PADDING = 2
const GAP = 2
const FOCUS_MARGIN = 10

const virtualizerOptions = computed(() => ({
  count: props.queue.length,
  getScrollElement: () => scrollContainerEl.value,
  estimateSize: (index: number) => {
    const id = props.queue[index]
    const isFocused = id === props.focusId
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
  const isFocused = props.queue[virtualItem.index] === props.focusId
  const width = isFocused ? virtualItem.size - FOCUS_MARGIN * 2 - GAP : virtualItem.size - GAP
  const translateX = isFocused ? virtualItem.start + FOCUS_MARGIN : virtualItem.start

  return {
    width: `${width}px`,
    height: `${props.height - PADDING * 2}px`,
    transform: `translateX(${translateX}px)`,
  }
}

// Force virtualizer remeasurement and auto-center the current active item
watch(
  [() => props.focusId, () => props.ratio],
  ([newId]) => {
    if (!newId) return
    const index = props.queue.indexOf(newId)
    if (index !== -1) {
      virtualizer.value.measure()
      nextTick(() => {
        virtualizer.value.scrollToIndex(index, {
          align: 'center',
          behavior: 'smooth',
        })
      })
    }
  },
  { immediate: true },
)

watch(
  () => props.queue,
  () => {
    virtualizer.value.measure()
  },
  { deep: true },
)
</script>

<template>
  <div ref="scrollContainer" class="gallery-container">
    <div
      class="gallery-inner"
      :style="{
        width: `${virtualizer.getTotalSize()}px`,
      }"
    >
      <thumbnail-img
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
  background-color: red;
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
  position: absolute;
  top: 0;
  left: 0;
  overflow: hidden;
  box-sizing: border-box;
  border-radius: 4px;
}
</style>
