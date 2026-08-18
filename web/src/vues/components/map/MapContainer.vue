<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { LngLatBounds, type MapOptions } from 'maplibre-gl'
import { Map as LibreMap } from 'maplibre-gl'
import BaseMap, { type StyleName } from '@/vues/components/map/BaseMap.vue'
import MapLayerSelector from '@/vues/components/map/MapLayerSelector.vue'
import MapDateFilter from '@/vues/components/map/MapDateFilter.vue'
import MapMarkersLayer, {
  type EmitClusterSelected,
  type EmitMarkerSelected,
} from '@/vues/components/map/layers/MapMarkersLayer.vue'
import MapDotsLayer from '@/vues/components/map/layers/MapDotsLayer.vue'
import MapHeatmapLayer from '@/vues/components/map/layers/MapHeatmapLayer.vue'
import { getLngLat } from '@/vues/components/map/layers/mapUtils.ts'
import { useStorage } from '@vueuse/core'
import libertyMapThumb from '@/assets/img/map-thumb/LIBERTY.png'
import darkColorfulMapThumb from '@/assets/img/map-thumb/DARK_COLORFUL.png'
import satelliteMapThumb from '@/assets/img/map-thumb/SATELLITE.png'
import terrainMapThumb from '@/assets/img/map-thumb/TERRAIN.png'
import watercolorMapThumb from '@/assets/img/map-thumb/WATERCOLOR.png'
import type { MapPhotosResponse, SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'

export type { EmitMarkerSelected, EmitClusterSelected }

const MAP_STYLES = [
  { key: 'LIBERTY', label: 'Light Map', thumb: libertyMapThumb },
  { key: 'SATELLITE', label: 'Satellite', thumb: satelliteMapThumb },
  { key: 'TERRAIN', label: 'Terrain', thumb: terrainMapThumb },
  { key: 'WATERCOLOR', label: 'Watercolor', thumb: watercolorMapThumb },
  { key: 'DARK_COLORFUL', label: 'Dark Map', thumb: darkColorfulMapThumb },
] as const

const DEFAULT_MAP_OPTIONS = {
  center: { lat: 40, lng: 0 },
  zoom: 3,
  attributionControl: { compact: true },
}

interface Props {
  mapPhotos: MapPhotosResponse | null
  loadCoord: { lat: number; lng: number } | null
}

export interface DateFilter {
  startDate: Date | null
  endDate: Date | null
  active: boolean
  startGranularity: 'month' | 'day'
  endGranularity: 'month' | 'day'
}

const props = defineProps<Props>()

const emit = defineEmits<{
  'visible-items-changed': [items: SimpleTimelineItem[]]
  'marker-selected': [data: EmitMarkerSelected]
  'cluster-selected': [data: EmitClusterSelected]
  'date-filter-change': [payload: { isDragging: boolean; dateFilter: DateFilter }]
}>()

// --- State & Storage ---
const mapMode = useStorage<'markers' | 'heatmap' | 'dots'>('mapLayerMode', 'markers')
const map = shallowRef<LibreMap | null>(null)
const photoIdToOrder = new Map<string, number>()

const dateFilter = ref<DateFilter>({
  startDate: null,
  endDate: null,
  active: false,
  startGranularity: 'month',
  endGranularity: 'month',
})

const mapOptions = ref<Omit<MapOptions, 'container' | 'style'> | null>(null)
const currentStyle = useStorage<StyleName>('mapCurrentStyle', 'LIBERTY')

// --- Computed Properties ---
const nextStyle = computed(() => {
  const currentIndex = MAP_STYLES.findIndex((s) => s.key === currentStyle.value)
  const nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % MAP_STYLES.length
  return MAP_STYLES[nextIndex]
})

function cycleStyle() {
  currentStyle.value = nextStyle.value.key
}

// --- Fetch Data & Handlers ---
function handleDateFilterChange(payload: { isDragging: boolean }) {
  emit('date-filter-change', {
    isDragging: payload.isDragging,
    dateFilter: dateFilter.value,
  })
}

// --- Map Lifecycle ---
function handleMapLoad(loadedMap: LibreMap) {
  map.value = loadedMap
}

function zoomToFitAll() {
  if (!map.value || !props.mapPhotos) return
  const locations = props.mapPhotos.items
  if (locations.length === 0) return

  if (locations.length === 1) {
    const [location] = locations
    map.value.flyTo({
      center: getLngLat(location!),
      zoom: 11,
    })
    return
  }

  const bounds = locations.reduce(
    (photoBounds, item) => {
      return photoBounds.extend([item.longitude, item.latitude])
    },
    new LngLatBounds(getLngLat(locations[0]!), getLngLat(locations[0]!)),
  )

  map.value.fitBounds(bounds, {
    padding: 80,
    maxZoom: 14,
    duration: 1200,
  })
}

function getInitialMapOptions(photos: MapPhotosResponse): Omit<MapOptions, 'container' | 'style'> {
  if (props.loadCoord) {
    return {
      ...DEFAULT_MAP_OPTIONS,
      center: props.loadCoord,
      zoom: 16,
    }
  }
  const locations = photos.items
  if (locations.length === 0) return DEFAULT_MAP_OPTIONS

  if (locations.length === 1) {
    const [location] = locations
    return {
      ...DEFAULT_MAP_OPTIONS,
      center: getLngLat(location),
      zoom: 11,
    }
  }

  const bounds = locations.reduce(
    (photoBounds, item) => {
      return photoBounds.extend([item.longitude, item.latitude])
    },
    new LngLatBounds(getLngLat(locations[0]), getLngLat(locations[0])),
  )

  return {
    ...DEFAULT_MAP_OPTIONS,
    bounds,
    fitBoundsOptions: {
      padding: 80,
      maxZoom: 14,
    },
  }
}

// --- Initialization ---
if (props.mapPhotos) {
  props.mapPhotos.items.forEach((p, index) => {
    if (p.item?.id) {
      photoIdToOrder.set(p.item.id, index)
    }
  })
  mapOptions.value = getInitialMapOptions(props.mapPhotos)
}

// --- Watchers ---
watch(
  () => props.mapPhotos,
  (newPhotos) => {
    if (newPhotos) {
      newPhotos.items.forEach((p, index) => {
        if (p.item?.id) {
          photoIdToOrder.set(p.item.id, index)
        }
      })
      if (!mapOptions.value) {
        mapOptions.value = getInitialMapOptions(newPhotos)
      }
    }
  },
  { deep: true },
)

watch(
  () => props.loadCoord,
  (newVal, oldVal) => {
    if (!map.value || !props.loadCoord) return
    if (JSON.stringify(newVal) === JSON.stringify(oldVal)) return
    map.value.flyTo({
      center: props.loadCoord,
      zoom: 17,
    })
  },
)

defineExpose({
  zoomToFitAll,
})
</script>

<template>
  <div class="map-container">
    <v-theme-provider with-background class="map-wrapper" theme="light">
      <base-map
        v-if="mapOptions"
        :map-style="currentStyle"
        class="map-instance"
        :map-options="mapOptions"
        @load="handleMapLoad"
      />
      <div v-else class="map-loading">
        <v-progress-circular indeterminate color="primary" />
      </div>

      <!-- Marker / Dots / Heatmap Layers -->
      <template v-if="map">
        <map-markers-layer
          v-if="mapMode === 'markers'"
          :map="map"
          :map-photos="props.mapPhotos"
          @visible-items-changed="emit('visible-items-changed', $event)"
          @marker-selected="emit('marker-selected', $event)"
          @cluster-selected="emit('cluster-selected', $event)"
        />
        <map-dots-layer
          v-else-if="mapMode === 'dots'"
          :map="map"
          :map-photos="props.mapPhotos"
          @visible-items-changed="emit('visible-items-changed', $event)"
        />
        <map-heatmap-layer
          v-else
          :map="map"
          :map-photos="props.mapPhotos"
          @visible-items-changed="emit('visible-items-changed', $event)"
        />
      </template>

      <!-- Date Range Filter -->
      <map-date-filter
        v-if="props.mapPhotos"
        :theme="currentStyle === 'DARK_COLORFUL' ? 'dark' : 'light'"
        :model-value="dateFilter"
        @change="handleDateFilterChange"
        @update:model-value="dateFilter = $event"
      />

      <!-- Map Style / Layer Selector -->
      <map-layer-selector
        v-if="mapOptions"
        :current-style="currentStyle"
        :map-mode="mapMode"
        :next-style="nextStyle"
        :map-styles="MAP_STYLES"
        @update:current-style="currentStyle = $event"
        @update:map-mode="mapMode = $event"
        @cycle-style="cycleStyle"
      />
    </v-theme-provider>
  </div>
</template>

<style scoped>
.map-container {
  width: 100%;
  height: 100%;
}

/* --- Map Wrapper & Controls --- */
.map-wrapper {
  width: 100%;
  height: 100%;
  position: relative;
}

.map-instance,
.map-instance > div {
  width: 100%;
  height: 100%;
}

.map-loading {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
}
</style>

<style>
/* Shared Map Media Popup Styles */
.map-media-popup {
  position: relative;
  border: 2px solid rgba(255, 255, 255, 0.86);
  border-radius: 12px;
  background: rgba(20, 20, 24, 0.78);
  box-shadow: 0 12px 28px rgba(0, 0, 0, 0.35);
  cursor: pointer;
  z-index: 20;
}

.map-media-popup::after {
  content: '';
  position: absolute;
  left: 50%;
  bottom: -14px;
  width: 0;
  height: 0;
  border-left: 13px solid transparent;
  border-right: 13px solid transparent;
  border-top: 14px solid rgba(255, 255, 255, 0.86);
  transform: translateX(-50%);
}

.map-media-popup::before {
  content: '';
  position: absolute;
  left: 50%;
  bottom: -11px;
  width: 0;
  height: 0;
  border-left: 10px solid transparent;
  border-right: 10px solid transparent;
  border-top: 11px solid rgba(20, 20, 24, 0.78);
  transform: translateX(-50%);
  z-index: 1;
}

.map-media-popup-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 10px;
}

.map-media-popup-close {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.45);
  color: white;
  cursor: pointer;
  font-size: 18px;
  display: grid;
  place-items: center;
  z-index: 1;
}

.map-media-popup-close:hover {
  background: rgba(0, 0, 0, 0.68);
}
</style>
