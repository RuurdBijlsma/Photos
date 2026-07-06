<script setup lang="ts">
import { computed, ref, useTemplateRef } from 'vue'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import MapContainer, {
  type DateFilter,
  type EmitClusterSelected,
  type EmitMarkerSelected,
} from '@/vues/components/map/MapContainer.vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { useEventListener, useResizeObserver, useStorage, useThrottleFn } from '@vueuse/core'
import type { MapPhotosResponse, SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import { useRoute } from 'vue-router'

const route = useRoute()

// --- State & Storage ---
const mapPhotos = ref<MapPhotosResponse | null>(null)

// Sidebar selection state (from MapContainer emissions)
const visibleItems = ref<SimpleTimelineItem[]>([])
const selectedClusterItems = ref<SimpleTimelineItem[] | null>(null)
const selectedMarkerKey = ref<string | null>(null)
const selectedPopupItem = ref<SimpleTimelineItem | null>(null)
const selectedLngLat = ref<[number, number] | null>(null)

// Photo ordering for sorting
const photoIdToOrder = new Map<string, number>()

// --- Layout & Resize Settings ---
const outerLayoutEl = useTemplateRef('outerLayout')
const mapContainerRef = useTemplateRef<InstanceType<typeof MapContainer>>('mapContainerRef')
const MIN_MAP_WIDTH = 400
const MIN_SIDEBAR_WIDTH = 200
const SIDEBAR_GAP = 5
const CLOSE_DRAG_THRESHOLD = 50

const sidebarOpen = useStorage('mapSidebarOpen', true)
const sidebarWidth = useStorage('mapSidebarWidth', window.innerWidth / 3)
const isResizingSidebar = ref(false)

// --- Computed Properties ---
const loadCoord = computed(() => {
  const lat = Number(route.query.lat)
  const lon = Number(route.query.lon)
  if (!lat || !lon) return null
  return { lat, lng: lon }
})

const timelineItems = computed(() => {
  const items = selectedClusterItems.value ?? visibleItems.value
  return items.slice().sort((a, b) => {
    const orderA = photoIdToOrder.get(a.id) ?? 0
    const orderB = photoIdToOrder.get(b.id) ?? 0
    return orderA - orderB
  })
})

const resolvedSidebarWidth = computed(() => {
  const maxWidth = getMaxSidebarWidth()
  return Math.round(Math.min(Math.max(sidebarWidth.value, MIN_SIDEBAR_WIDTH), maxWidth))
})

const layoutStyle = computed(() => ({
  '--map-sidebar-width': sidebarOpen.value ? `${resolvedSidebarWidth.value}px` : '0px',
}))

// --- Fetch Data ---
async function fetchMapPhotos(start: Date | null, end: Date | null) {
  const startStr = start?.toISOString()
  const endStr = end?.toISOString()

  try {
    const loadedPhotos = await mediaItemService.listMapPhotos(startStr, endStr)
    photoIdToOrder.clear()
    loadedPhotos.items.forEach((p, index) => {
      if (p.item?.id) {
        photoIdToOrder.set(p.item.id, index)
      }
    })
    mapPhotos.value = loadedPhotos
    clearMarkerSelection()
  } catch (err) {
    console.error('Failed to list map photos with date filter:', err)
  }
}

const throttledFetchMapPhotos = useThrottleFn(fetchMapPhotos, 100)

function handleDateFilterChange(payload: { isDragging: boolean; dateFilter: DateFilter }) {
  const start = payload.dateFilter.active ? payload.dateFilter.startDate : null
  const end = payload.dateFilter.active ? payload.dateFilter.endDate : null

  if (payload.isDragging) {
    // While dragging, use throttled fetch to limit updates
    throttledFetchMapPhotos(start, end)
  } else {
    // When drag is released, do an immediate unthrottled fetch for final result
    fetchMapPhotos(start, end)
  }
}

// --- Layout Sizing Controls ---
function closeSidebar() {
  sidebarOpen.value = false
}

function openSidebar() {
  sidebarOpen.value = true
}

function clearMarkerSelection() {
  selectedClusterItems.value = null
  selectedMarkerKey.value = null
  selectedPopupItem.value = null
  selectedLngLat.value = null
}

function startSidebarResize(e: MouseEvent) {
  if (!sidebarOpen.value || !outerLayoutEl.value) return
  e.preventDefault()
  isResizingSidebar.value = true
}

function updateSidebarResize(clientX: number) {
  if (!outerLayoutEl.value) return
  const rect = outerLayoutEl.value.getBoundingClientRect()
  if (clientX >= rect.right - CLOSE_DRAG_THRESHOLD) {
    closeSidebar()
    isResizingSidebar.value = false
    return
  }

  const rawWidth = rect.right - clientX
  sidebarWidth.value = Math.round(
    Math.min(Math.max(rawWidth, MIN_SIDEBAR_WIDTH), getMaxSidebarWidth()),
  )
}

function getMaxSidebarWidth() {
  const layoutWidth = outerLayoutEl.value?.getBoundingClientRect().width ?? window.innerWidth
  return Math.max(MIN_SIDEBAR_WIDTH, layoutWidth - MIN_MAP_WIDTH - SIDEBAR_GAP)
}

useEventListener(window, 'mousemove', (e: MouseEvent) => {
  if (!isResizingSidebar.value) return
  e.preventDefault()
  updateSidebarResize(e.clientX)
})

useEventListener(window, 'mouseup', () => {
  isResizingSidebar.value = false
})

useResizeObserver(outerLayoutEl, () => {
  sidebarWidth.value = resolvedSidebarWidth.value
})

// --- Data Loading ---
mediaItemService.listMapPhotos().then((loadedPhotos) => {
  loadedPhotos.items.forEach((p, index) => {
    if (p.item?.id) {
      photoIdToOrder.set(p.item.id, index)
    }
  })
  mapPhotos.value = loadedPhotos
})

function onMarkerSelected(data: EmitMarkerSelected) {
  selectedMarkerKey.value = data.key
  selectedLngLat.value = data.coords
}
function onClusterSelected(data: EmitClusterSelected) {
  selectedClusterItems.value = data.items
  selectedPopupItem.value = data.item
}
</script>

<template>
  <div
    ref="outerLayout"
    class="outer-layout"
    :class="{
      'sidebar-closed': !sidebarOpen,
      'sidebar-resizing': isResizingSidebar,
    }"
    :style="layoutStyle"
  >
    <main-layout-container class="map-layout">
      <map-container
        ref="mapContainerRef"
        :map-photos="mapPhotos"
        :load-coord="loadCoord"
        @visible-items-changed="visibleItems = $event"
        @marker-selected="onMarkerSelected"
        @cluster-selected="onClusterSelected"
        @date-filter-change="handleDateFilterChange"
      />
      <v-btn
        v-if="!sidebarOpen"
        class="open-sidebar-btn"
        icon
        density="comfortable"
        color="primary"
        variant="elevated"
        @click="openSidebar"
        v-tooltip="{ location: 'left', text: 'Open sidebar' }"
      >
        <v-icon size="20" icon="mdi-chevron-left" />
      </v-btn>
    </main-layout-container>

    <div
      class="sidebar-resize-handle"
      :class="{ disabled: !sidebarOpen }"
      @mousedown="startSidebarResize"
    />

    <simple-timeline
      v-if="mapPhotos"
      hide-drop-shadow
      class="timeline"
      :timeline-items="timelineItems"
      view-link="/map/view/"
    >
      <div class="timeline-header">
        <div>
          <template v-if="selectedClusterItems">
            <div class="photo-count-header">
              <h2>Cluster</h2>
              <span class="photo-count">{{ timelineItems.length.toLocaleString() }}</span>
            </div>
            <v-btn
              density="compact"
              variant="plain"
              color="primary"
              class="return-cluster-button"
              rounded
              @click="clearMarkerSelection"
              prepend-icon="mdi-chevron-left"
            >
              Deselect
            </v-btn>
          </template>
          <template v-else>
            <div class="photo-count-header">
              <h2>In View</h2>
              <span class="photo-count">{{ timelineItems.length.toLocaleString() }}</span>
            </div>
          </template>
        </div>
        <v-spacer />
        <v-btn
          icon
          density="compact"
          color="primary"
          variant="text"
          @click="closeSidebar"
          v-tooltip="{ location: 'top', text: 'Close sidebar' }"
        >
          <v-icon size="18" icon="mdi-chevron-right" />
        </v-btn>
      </div>

      <div v-if="timelineItems.length === 0" class="map-empty-state">
        <v-icon icon="mdi-map-search-outline" size="120" class="map-empty-icon" />
        <h3 class="map-empty-title">No items in this area</h3>
        <p class="map-empty-description">
          Move or zoom the map to find photos taken in other locations.
        </p>
        <v-btn
          color="primary"
          variant="tonal"
          rounded="xl"
          class="map-empty-button"
          prepend-icon="mdi-image-multiple-outline"
          @click="mapContainerRef?.zoomToFitAll()"
        >
          View All Photos
        </v-btn>
      </div>
    </simple-timeline>
  </div>
</template>

<style>
/* --- Outer Layout Framework --- */
.outer-layout {
  height: 100%;
  width: 100%;
  display: grid;
  grid-template-columns: minmax(400px, 1fr) 5px var(--map-sidebar-width);
  transition: grid-template-columns 0.22s ease;
}

.outer-layout.sidebar-resizing {
  transition: none;
  user-select: none;
  cursor: col-resize;
}

.outer-layout.sidebar-closed {
  grid-template-columns: minmax(400px, 1fr) 0 0;
}

.map-layout {
  width: 100% !important;
  max-width: 100% !important;
  min-width: 0;
  overflow: hidden;
  position: relative;
}

.open-sidebar-btn {
  position: absolute !important;
  top: 40px;
  right: 40px;
  z-index: 2;
}

.timeline {
  width: var(--map-sidebar-width) !important;
  min-width: 0;
  overflow: hidden;
  transition:
    width 0.22s ease,
    opacity 0.18s ease;
}

.sidebar-resizing .timeline {
  transition: none;
}

.sidebar-closed .timeline {
  opacity: 0;
  pointer-events: none;
}

.sidebar-resize-handle {
  width: 5px;
  height: 100%;
  cursor: col-resize;
  z-index: 3;
}

.sidebar-resize-handle.disabled {
  opacity: 0;
  pointer-events: none;
}

/* --- Timeline Sidebar Elements --- */
.timeline-header {
  padding: 15px 20px;
  margin-bottom: 10px;
  display: flex;
  align-items: center;
}

.timeline h2 {
  font-weight: 500;
  font-size: 20px;
  color: rgba(var(--v-theme-on-surface-variant));
  margin: 0;
}

.return-cluster-button {
  margin-left: -15px;
  transform: scale(0.9);
}

.photo-count-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.photo-count {
  color: rgba(var(--v-theme-on-surface-variant), 0.7);
  font-weight: 500;
  font-size: 20px;
}

/* --- Empty Search State --- */
.map-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 40px 20px;
  text-align: center;
  flex: 1;
}

.map-empty-icon {
  color: rgba(var(--v-theme-on-surface), 0.2);
}

.map-empty-title {
  font-size: 18px;
  font-weight: 600;
  color: rgba(var(--v-theme-on-surface));
  margin: 0;
}

.map-empty-description {
  font-size: 14px;
  color: rgba(var(--v-theme-on-surface-variant), 0.8);
  margin: 0;
  max-width: 280px;
}

.map-empty-button {
  margin-top: 8px;
}
</style>
