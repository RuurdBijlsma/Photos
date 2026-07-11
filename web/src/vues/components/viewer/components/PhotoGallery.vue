<script setup lang="ts">
import { computed, nextTick, useTemplateRef, watch } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
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
    return (
      (isFocused ? (props.height - PADDING * 2) * props.ratio : props.height * NON_FOCUS_RATIO) +
      GAP
    )
  },
  horizontal: true,
  overscan: 10,
}))

const virtualizer = useVirtualizer(virtualizerOptions)

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
        :style="{
          width: `${virtualItem.size - GAP}px`,
          height: `${height - PADDING * 2}px`,
          transform: `translateX(${virtualItem.start}px)`,
        }"
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
