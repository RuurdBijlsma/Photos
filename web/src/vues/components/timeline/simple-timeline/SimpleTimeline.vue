<script setup lang="ts">
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import { computed, nextTick, ref, shallowRef, useTemplateRef, watch } from 'vue'
import type { SimpleLayoutRow, TimelineContext } from '@/scripts/types/timeline/layout.ts'
import { getThumbnailHeight } from '@/scripts/utils.ts'
import { useDebounceFn, useEventListener, useResizeObserver, useThrottleFn } from '@vueuse/core'
import { useVirtualizer } from '@tanstack/vue-virtual'
import VirtualSimpleRow from '@/vues/components/timeline/simple-timeline/VirtualSimpleRow.vue'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import SelectionOverlay from '@/vues/components/timeline/timeline-components/SelectionOverlay.vue'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import ReorderGridRow from '@/vues/components/timeline/simple-timeline/ReorderGridRow.vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'

const props = withDefaults(
  defineProps<{
    timelineItems: SimpleTimelineItem[]
    viewLink: string
    loadingMore?: boolean
    context?: TimelineContext
    isManualOrderMode?: boolean
    hideDropShadow?: boolean
    idealRowHeight?: number
    hideScrollBar?: boolean
  }>(),
  {
    context: () => ({}),
    isManualOrderMode: false,
    hideDropShadow: false,
    hideScrollBar: false,
    idealRowHeight: 330,
  },
)

const emit = defineEmits(['loadMore', 'reorder'])

const viewPhotoStore = useViewPhotoStore()
const selectionStore = useSelectionStore()
const settings = useSettingStore()

const localItemsOrder = ref<SimpleTimelineItem[]>([])
let scrollInterval: number | null = null

watch(
  () => props.timelineItems,
  (newItems) => {
    if (!props.isManualOrderMode) {
      localItemsOrder.value = [...newItems]
    }
  },
  { immediate: true },
)

function onReorder({
  sourceId,
  targetId,
  position,
}: {
  sourceId: string
  targetId: string
  position: 'before' | 'after'
}) {
  const items = [...localItemsOrder.value]
  const sourceIndex = items.findIndex((i) => i.id === sourceId)
  if (sourceIndex === -1) return
  const [movedItem] = items.splice(sourceIndex, 1)

  let targetIndex = items.findIndex((i) => i.id === targetId)
  if (targetIndex === -1) return

  if (position === 'after') targetIndex++
  items.splice(targetIndex, 0, movedItem!)

  localItemsOrder.value = items
  emit('reorder', items)
}

const MAX_SIZE_MULTIPLIER = 1.5
const ITEM_GAP = 2
const MIN_THUMB_HEIGHT = 20
const SNAP_MARGIN = 20

const gridLayout = shallowRef<SimpleLayoutRow[]>([])
const scrollContainerEl = useTemplateRef('scrollContainer')
const scrollTrackEl = useTemplateRef('scrollTrack')
const customSlotEl = useTemplateRef('customSlot')
const customSlotHeight = ref(0)
const containerHeight = ref(0)
const containerWidth = ref(0)
const contentHeight = ref(0)
const trackHeight = ref(0)
const scrollTop = ref(0)
const isScrollingFast = ref(false)
const thumbHeight = computed(() => {
  if (contentHeight.value <= containerHeight.value || containerHeight.value === 0) return 0
  const ratio = containerHeight.value / contentHeight.value
  const calculatedHeight = trackHeight.value * ratio
  return Math.max(calculatedHeight, MIN_THUMB_HEIGHT)
})
const thumbTranslateY = computed(() => {
  const maxScroll = contentHeight.value - containerHeight.value
  const maxThumbTravel = trackHeight.value - thumbHeight.value
  if (maxScroll <= 0 || maxThumbTravel <= 0) return 0
  const scrollRatio = scrollTop.value / maxScroll
  const clampedRatio = Math.min(1, Math.max(0, scrollRatio))
  return clampedRatio * maxThumbTravel
})
const showScrollbar = computed(() => {
  return contentHeight.value > containerHeight.value && containerHeight.value > 0
})
const virtualizerOptions = computed(() => ({
  count: gridLayout.value.length,
  getScrollElement: () => scrollContainerEl.value,
  estimateSize: (index: number) => {
    const row = gridLayout.value[index]
    if (!row) return 0
    return row.height + ITEM_GAP
  },
  overscan: 5,
}))
const rowVirtualizer = useVirtualizer(virtualizerOptions)

let isDragging = false
let lastScrollTop = 0
let dragStartOffsetY = 0

function calculateLayout(timelineItems: SimpleTimelineItem[], containerWidth: number) {
  if (timelineItems.length === 0 || containerWidth === 0) return { rows: [], totalHeight: 0 }
  const layoutRows: SimpleLayoutRow[] = []
  let rowWidth = 0
  let offsetTop = 0
  let rowItems: SimpleTimelineItem[] = []

  for (const [i, item] of timelineItems.entries()) {
    rowItems.push(item)
    const gapSize = (rowItems.length - 1) * ITEM_GAP
    rowWidth += props.idealRowHeight * item.ratio
    if (rowWidth + gapSize > containerWidth) {
      const sizeMultiplier = Math.min((containerWidth - gapSize) / rowWidth, MAX_SIZE_MULTIPLIER)
      const rowHeight = props.idealRowHeight * sizeMultiplier
      layoutRows.push({
        items: rowItems,
        height: rowHeight,
        key: layoutRows.length.toString(),
        offsetTop,
        thumbnailSize: getThumbnailHeight(rowHeight),
        firstRow: layoutRows.length === 0,
        lastRow: i === timelineItems.length - 1,
      })
      rowItems = []
      rowWidth = 0
      offsetTop += Math.round(rowHeight) + ITEM_GAP
    }
  }

  if (rowItems.length > 0) {
    let sizeMultiplier = Math.min(containerWidth / rowWidth, MAX_SIZE_MULTIPLIER)
    // If full row width can be reached width size multiplier, then use it, otherwise dont increase size
    if (rowWidth * sizeMultiplier < containerWidth) {
      sizeMultiplier = 1
    }
    const rowHeight = props.idealRowHeight * sizeMultiplier
    layoutRows.push({
      items: rowItems,
      height: rowHeight,
      key: layoutRows.length.toString(),
      offsetTop,
      thumbnailSize: getThumbnailHeight(rowHeight),
      firstRow: layoutRows.length === 0,
      lastRow: true,
    })
    offsetTop += Math.round(rowHeight) + ITEM_GAP
  }

  // Add indices to items for reordering logic
  let currentIndex = 0
  for (const row of layoutRows) {
    for (const item of row.items as (SimpleTimelineItem & { index: number })[]) {
      item.index = currentIndex++
    }
  }

  return {
    rows: layoutRows,
    totalHeight: offsetTop,
  }
}

function applyScrollFromMouseY(clientY: number) {
  if (!scrollTrackEl.value || !scrollContainerEl.value) return
  const trackRect = scrollTrackEl.value.getBoundingClientRect()
  const trackTop = trackRect.top
  let newThumbY = clientY - dragStartOffsetY - trackTop
  const maxThumbTravel = trackHeight.value - thumbHeight.value
  newThumbY = Math.max(0, Math.min(newThumbY, maxThumbTravel))
  const scrollRatio = newThumbY / maxThumbTravel
  const maxScroll = contentHeight.value - containerHeight.value
  const targetScrollTop = scrollRatio * maxScroll
  scrollContainerEl.value.scrollTop = targetScrollTop
  scrollTop.value = targetScrollTop
}

function handleMouseDown(e: MouseEvent) {
  if (!scrollTrackEl.value || !showScrollbar.value) return
  e.preventDefault()
  const trackRect = scrollTrackEl.value.getBoundingClientRect()
  const clickYRelative = e.clientY - trackRect.top
  const currentThumbY = thumbTranslateY.value
  const currentThumbH = thumbHeight.value
  const distToTop = clickYRelative - currentThumbY
  const distToBottom = clickYRelative - (currentThumbY + currentThumbH)
  if (clickYRelative >= currentThumbY && clickYRelative <= currentThumbY + currentThumbH) {
    dragStartOffsetY = clickYRelative - currentThumbY
  } else if (distToTop >= -SNAP_MARGIN && distToTop < 0) {
    dragStartOffsetY = 0
  } else if (distToBottom > 0 && distToBottom <= SNAP_MARGIN) {
    dragStartOffsetY = currentThumbH
  } else {
    dragStartOffsetY = currentThumbH / 2
    applyScrollFromMouseY(e.clientY)
  }

  isDragging = true
}

function handleFastScroll(currentY: number) {
  const scrollDelta = Math.abs(currentY - lastScrollTop)
  lastScrollTop = currentY
  if (scrollDelta > 500) {
    if (!isScrollingFast.value) {
      isScrollingFast.value = true
    }
    stopScrollingFast()
  } else if (isScrollingFast.value && scrollDelta > 200) stopScrollingFast()
}

const onScroll = useThrottleFn((e: Event) => {
  const target = e.target as HTMLElement
  contentHeight.value = target.scrollHeight
  scrollTop.value = target.scrollTop
  handleFastScroll(target.scrollTop)

  if (scrollTop.value + containerHeight.value > contentHeight.value - 1000) {
    emit('loadMore')
  }
}, 16)

watch(isScrollingFast, () => console.log('isScrollingFast', isScrollingFast.value))

const stopScrollingFast = useDebounceFn(() => {
  isScrollingFast.value = false
}, 150)

function scrollToTop() {
  if (scrollContainerEl.value) {
    scrollContainerEl.value.scrollTop = 0
  }
}

function handleDragOver(e: DragEvent) {
  if (!props.isManualOrderMode || !scrollContainerEl.value) return
  const rect = scrollContainerEl.value.getBoundingClientRect()
  const viewportHeight = window.innerHeight
  const threshold = 120

  // Calculate effective boundaries within the viewport
  const containerTop = rect.top
  const containerBottom = Math.min(rect.bottom, viewportHeight)
  const mouseY = e.clientY

  if (scrollInterval) {
    cancelAnimationFrame(scrollInterval)
    scrollInterval = null
  }

  let speed = 0
  if (mouseY < containerTop + threshold) {
    // Scroll up
    const dist = containerTop + threshold - mouseY
    speed = -Math.max(2, dist / 4)
  } else if (mouseY > containerBottom - threshold) {
    // Scroll down
    const dist = mouseY - (containerBottom - threshold)
    speed = Math.max(2, dist / 4)
  }

  if (speed !== 0) {
    const maxSpeed = 35
    const finalSpeed = Math.sign(speed) * Math.min(Math.abs(speed), maxSpeed)
    scrollInterval = requestAnimationFrame(function scroll() {
      if (scrollContainerEl.value) {
        scrollContainerEl.value.scrollTop += finalSpeed
        scrollInterval = requestAnimationFrame(scroll)
      }
    })
  }
}

function handleDragEnd() {
  if (scrollInterval) cancelAnimationFrame(scrollInterval)
  scrollInterval = null
}

useEventListener(window, 'dragend', handleDragEnd)
useEventListener(window, 'drop', handleDragEnd)
useEventListener(window, 'blur', handleDragEnd)
useEventListener(document, 'visibilitychange', () => {
  if (document.hidden) handleDragEnd()
})

defineExpose({
  scrollToTop,
  setOrder(items: SimpleTimelineItem[]) {
    localItemsOrder.value = [...items]
  },
})

useResizeObserver(scrollContainerEl, (entries) => {
  if (entries[0]) {
    const rect = entries[0].contentRect
    containerWidth.value = rect.width
    containerHeight.value = rect.height
  }
})
useResizeObserver(scrollTrackEl, (entries) => {
  if (entries[0]) {
    trackHeight.value = entries[0].contentRect.height
  }
})
useResizeObserver(customSlotEl, (entries) => {
  if (entries[0]) {
    customSlotHeight.value = entries[0].contentRect.height
  }
})

watch([localItemsOrder, containerWidth, () => props.idealRowHeight], () => {
  const { rows, totalHeight } = calculateLayout(localItemsOrder.value, containerWidth.value)
  gridLayout.value = rows
  contentHeight.value = totalHeight + customSlotHeight.value
  nextTick(() => rowVirtualizer.value.measure())
})

watch(
  () => props.timelineItems,
  () => {
    const ids = props.timelineItems.map((item) => item.id)
    viewPhotoStore.ids = ids
    viewPhotoStore.viewLink = props.viewLink
    selectionStore.allIds = ids
  },
  { immediate: true },
)

useEventListener(window, 'mousemove', (e: MouseEvent) => {
  if (!isDragging) return
  e.preventDefault()
  applyScrollFromMouseY(e.clientY)
})
useEventListener(window, 'mouseup', () => {
  isDragging = false
})
</script>

<template>
  <div class="simple-timeline">
    <main-layout-container :hide-drop-shadow="hideDropShadow" :ignore-scroll-bar="hideScrollBar">
      <selection-overlay v-if="context" :context="context" />
      <teleport to="body">
        <router-view />
      </teleport>

      <div
        class="scroll-container"
        ref="scrollContainer"
        @scroll.passive="onScroll"
        @dragover="handleDragOver"
        @dragend="handleDragEnd"
        @drop="handleDragEnd"
      >
        <div ref="customSlot">
          <slot></slot>
        </div>
        <div
          class="grid"
          :style="{
            height: `${rowVirtualizer.getTotalSize()}px`,
            width: '100%',
            position: 'relative',
          }"
        >
          <div
            v-for="virtualRow in rowVirtualizer.getVirtualItems()"
            :key="String(virtualRow.key)"
            :style="{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              transform: `translateY(${virtualRow.start}px)`,
            }"
          >
            <reorder-grid-row
              v-if="isManualOrderMode && gridLayout[virtualRow.index]"
              :item="gridLayout[virtualRow.index]!"
              :container-width="containerWidth"
              :item-gap="ITEM_GAP"
              @reorder="onReorder"
            />
            <virtual-simple-row
              v-else-if="gridLayout[virtualRow.index]"
              :item="gridLayout[virtualRow.index]!"
              :container-width="containerWidth"
              :item-gap="ITEM_GAP"
              :is-scrolling-fast="isScrollingFast"
              :view-link="viewLink"
              :async-decoding="settings.asyncImageDecoding"
            />
          </div>
          <div v-if="loadingMore" class="loading-more">
            <v-progress-circular indeterminate color="primary" size="32" />
          </div>
        </div>
        <div v-if="isManualOrderMode" class="reorder-bottom-spacer" />
      </div>
    </main-layout-container>

    <!-- Scrollbar Track -->
    <div
      v-if="!hideScrollBar"
      class="timeline-scroll"
      ref="scrollTrack"
      @mousedown="handleMouseDown"
      v-show="showScrollbar"
    >
      <div class="scroll-track"></div>
      <div
        class="scroll-thumb"
        :style="{
          height: `${thumbHeight}px`,
          transform: `translateY(${thumbTranslateY}px)`,
        }"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.simple-timeline {
  width: 100%;
  height: 100%;
  display: flex;
  --item-gap: calc(v-bind(ITEM_GAP) * 1px);
}

.scroll-container {
  height: 100%;
  width: 100%;
  overflow-y: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.scroll-container::-webkit-scrollbar {
  display: none;
}

.grid {
  border-radius: 25px;
  overflow: hidden;
}

.timeline-scroll {
  width: 50px;
  height: 100%;
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  user-select: none;
}

.scroll-track {
  background-color: rgba(var(--v-theme-on-surface), 0.08);
  width: 10px;
  height: 100%;
  position: absolute;
  right: 3px;
  top: 0;
  border-radius: 5px;
}

.scroll-thumb {
  background-color: rgb(var(--v-theme-primary));
  position: absolute;
  top: 0;
  right: 3px;
  width: 10px;
  border-radius: 5px;
  will-change: transform;
  transform: translateZ(0);
  pointer-events: none;
}

.loading-more {
  display: flex;
  justify-content: center;
  padding: 40px;
  width: 100%;
}

.reorder-bottom-spacer {
  height: 75px;
  width: 100%;
}
</style>
