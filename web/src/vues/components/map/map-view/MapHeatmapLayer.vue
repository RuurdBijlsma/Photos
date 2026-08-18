<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import type { GeoJSONSource, Map as LibreMap, MapSourceDataEvent } from 'maplibre-gl'
import type { MapPhotosResponse, SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import { HEATMAP_CONFIG } from '@/scripts/mapUtils/heatmapConfig.ts'
import {
  createPhotosGeoJson,
  getItemFromProperties,
} from '@/scripts/mapUtils/mapUtils.ts'
import { useDebounceFn } from '@vueuse/core'

const props = defineProps<{
  map: LibreMap
  mapPhotos: MapPhotosResponse | null
}>()

const emit = defineEmits<{
  'visible-items-changed': [items: SimpleTimelineItem[]]
}>()

const currentVisibleIds = new Set<string>()
const visibleItems = ref<SimpleTimelineItem[]>([])

function addHeatmapLayers(loadedMap: LibreMap) {
  // Heatmap Layer using configured intensity, color bands, and radius curves
  loadedMap.addLayer({
    id: 'photos-heat',
    type: 'heatmap',
    source: 'photos',
    maxzoom: HEATMAP_CONFIG.heatmapMaxZoom,
    paint: {
      'heatmap-intensity': [
        'interpolate',
        ['linear'],
        ['zoom'],
        ...HEATMAP_CONFIG.intensity.flat(),
      ],
      'heatmap-color': [
        'interpolate',
        ['linear'],
        ['heatmap-density'],
        ...HEATMAP_CONFIG.colorStops.flat(),
      ],
      'heatmap-radius': ['interpolate', ['linear'], ['zoom'], ...HEATMAP_CONFIG.radius.flat()],
      'heatmap-opacity': ['interpolate', ['linear'], ['zoom'], ...HEATMAP_CONFIG.opacity.flat()],
    },
  })

  // Exact point marker circles mapping to the zoom configuration
  loadedMap.addLayer({
    id: 'photos-point',
    type: 'circle',
    source: 'photos',
    minzoom: HEATMAP_CONFIG.pointMinZoom,
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], ...HEATMAP_CONFIG.point.radius.flat()],
      'circle-color': HEATMAP_CONFIG.point.color,
      'circle-stroke-color': HEATMAP_CONFIG.point.strokeColor,
      'circle-stroke-width': HEATMAP_CONFIG.point.strokeWidth,
      'circle-opacity': [
        'interpolate',
        ['linear'],
        ['zoom'],
        ...HEATMAP_CONFIG.point.opacity.flat(),
      ],
    },
  })

  // Invisible layer used solely to query visible items in the viewport
  loadedMap.addLayer({
    id: 'photos-helper',
    type: 'circle',
    source: 'photos',
    paint: {
      'circle-radius': 10,
      'circle-opacity': 0,
    },
  })
}

function syncVisibleHeatmapItems() {
  const loadedMap = props.map
  if (!loadedMap.getLayer('photos-helper')) return

  const helperFeatures = loadedMap.queryRenderedFeatures({ layers: ['photos-helper'] })
  if (helperFeatures.length === 0 && visibleItems.value.length === 0) {
    return
  }

  const newItems: SimpleTimelineItem[] = []
  const newIds = new Set<string>()

  for (const feature of helperFeatures) {
    const id = feature.properties?.id
    if (id && !newIds.has(id)) {
      const item = getItemFromProperties(feature.properties)
      if (item) {
        newIds.add(id)
        newItems.push(item)
      }
    }
  }

  // Skip updates if the on-screen set is identical
  let hasChanged = newIds.size !== currentVisibleIds.size
  if (!hasChanged) {
    for (const id of newIds) {
      if (!currentVisibleIds.has(id)) {
        hasChanged = true
        break
      }
    }
  }

  if (hasChanged) {
    currentVisibleIds.clear()
    newIds.forEach((id) => currentVisibleIds.add(id))
    visibleItems.value = newItems
    emit('visible-items-changed', newItems)
  }
}

const debouncedUpdate = useDebounceFn(syncVisibleHeatmapItems, 80)

function handleSourceData(e: MapSourceDataEvent) {
  if (e.sourceId === 'photos' && e.isSourceLoaded) debouncedUpdate()
}

function setupResources() {
  cleanupResources()

  if (props.mapPhotos) {
    props.map.addSource('photos', {
      type: 'geojson',
      data: createPhotosGeoJson(props.mapPhotos),
    })
  }

  addHeatmapLayers(props.map)
  props.map.once('idle', () => {
    syncVisibleHeatmapItems()
  })
}

function cleanupResources() {
  currentVisibleIds.clear()
  visibleItems.value = []

  const layersToRemove = ['photos-heat', 'photos-point', 'photos-helper']
  layersToRemove.forEach((layerId) => {
    if (props.map.getLayer(layerId)) props.map.removeLayer(layerId)
  })

  if (props.map.getSource('photos')) {
    props.map.removeSource('photos')
  }
}

function handleStyleLoad() {
  setupResources()
}

onMounted(() => {
  setupResources()
  props.map.on('zoomend', debouncedUpdate)
  props.map.on('moveend', debouncedUpdate)
  props.map.on('sourcedata', handleSourceData)
  props.map.on('style.load', handleStyleLoad)
})

onUnmounted(() => {
  props.map.off('zoomend', debouncedUpdate)
  props.map.off('moveend', debouncedUpdate)
  props.map.off('sourcedata', handleSourceData)
  props.map.off('style.load', handleStyleLoad)
  cleanupResources()
})

watch(
  () => props.mapPhotos,
  (newPhotos) => {
    if (!props.map) return
    const source = props.map.getSource('photos') as GeoJSONSource | undefined
    if (source && newPhotos) {
      source.setData(createPhotosGeoJson(newPhotos))
      props.map.triggerRepaint()
      setTimeout(() => {
        debouncedUpdate()
      }, 75)
    } else if (newPhotos) {
      setupResources()
    }
  },
  { deep: true },
)
</script>

<template>
  <!-- Renderless layer component directly controlling MapLibre canvas -->
  <slot />
</template>
