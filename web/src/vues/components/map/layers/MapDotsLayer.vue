<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import type { GeoJSONSource, Map as LibreMap, MapMouseEvent, MapSourceDataEvent } from 'maplibre-gl'
import type * as GeoJSON from 'geojson'
import type { MapPhotosResponse, SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import {
  createPhotosGeoJson,
  getItemFromProperties,
} from '@/vues/components/map/layers/mapUtils.ts'
import { MapMediaPopupController } from '@/vues/components/map/layers/mapPopup.ts'
import { useDebounceFn } from '@vueuse/core'
import { useRoute, useRouter } from 'vue-router'

const props = defineProps<{
  map: LibreMap
  mapPhotos: MapPhotosResponse | null
}>()

const emit = defineEmits<{
  'visible-items-changed': [items: SimpleTimelineItem[]]
}>()

const route = useRoute()
const router = useRouter()

const popupController = new MapMediaPopupController()
const currentVisibleIds = new Set<string>()
const visibleItems = ref<SimpleTimelineItem[]>([])

function addDotsLayer(loadedMap: LibreMap) {
  loadedMap.addLayer({
    id: 'photos-dots-circle',
    type: 'circle',
    source: 'photos-dots',
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 1, 4, 8, 5, 13, 7, 16, 9],
      'circle-color': 'rgb(80, 30, 120)',
      'circle-stroke-color': '#ffffff',
      'circle-stroke-width': 1.5,
      'circle-opacity': 0.85,
      'circle-stroke-opacity': 0.9,
    },
  })
}

function syncVisibleDots() {
  const loadedMap = props.map
  if (!loadedMap.getLayer('photos-dots-circle')) return

  const features = loadedMap.queryRenderedFeatures({ layers: ['photos-dots-circle'] })
  if (features.length === 0 && visibleItems.value.length === 0) {
    return
  }

  const newItems: SimpleTimelineItem[] = []
  const newIds = new Set<string>()

  for (const feature of features) {
    const id = feature.properties?.id
    if (id && !newIds.has(id)) {
      const item = getItemFromProperties(feature.properties)
      if (item) {
        newIds.add(id)
        newItems.push(item)
      }
    }
  }

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

const debouncedUpdate = useDebounceFn(syncVisibleDots, 80)

function handleSourceData(e: MapSourceDataEvent) {
  if (e.sourceId === 'photos-dots' && e.isSourceLoaded) debouncedUpdate()
}

function handleMouseEnter() {
  props.map.getCanvas().style.cursor = 'pointer'
}

function handleMouseLeave() {
  props.map.getCanvas().style.cursor = ''
}

function handleDotClick(e: MapMouseEvent) {
  const features = props.map.queryRenderedFeatures(e.point, { layers: ['photos-dots-circle'] })
  if (!features || features.length === 0) return

  const feature = features[0]
  const item = getItemFromProperties(feature.properties)
  if (!item) return

  const coords = (feature.geometry as GeoJSON.Point).coordinates as [number, number]

  popupController.show({
    map: props.map,
    item,
    coords,
    router,
    query: route.query,
    offset: [0, -10],
  })
}

function handleMapClick(e: MapMouseEvent) {
  const features = props.map.queryRenderedFeatures(e.point, { layers: ['photos-dots-circle'] })
  if (features.length === 0) {
    popupController.close()
  }
}

function setupResources() {
  cleanupResources()

  if (props.mapPhotos) {
    props.map.addSource('photos-dots', {
      type: 'geojson',
      data: createPhotosGeoJson(props.mapPhotos),
    })
  }

  addDotsLayer(props.map)
  props.map.once('idle', () => {
    syncVisibleDots()
  })
}

function cleanupResources() {
  popupController.close()
  currentVisibleIds.clear()
  visibleItems.value = []

  if (props.map.getLayer('photos-dots-circle')) {
    props.map.removeLayer('photos-dots-circle')
  }

  if (props.map.getSource('photos-dots')) {
    props.map.removeSource('photos-dots')
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
  props.map.on('mouseenter', 'photos-dots-circle', handleMouseEnter)
  props.map.on('mouseleave', 'photos-dots-circle', handleMouseLeave)
  props.map.on('click', 'photos-dots-circle', handleDotClick)
  props.map.on('click', handleMapClick)
  props.map.on('style.load', handleStyleLoad)
})

onUnmounted(() => {
  props.map.off('zoomend', debouncedUpdate)
  props.map.off('moveend', debouncedUpdate)
  props.map.off('sourcedata', handleSourceData)
  props.map.off('mouseenter', 'photos-dots-circle', handleMouseEnter)
  props.map.off('mouseleave', 'photos-dots-circle', handleMouseLeave)
  props.map.off('click', 'photos-dots-circle', handleDotClick)
  props.map.off('click', handleMapClick)
  props.map.off('style.load', handleStyleLoad)
  cleanupResources()
})

watch(
  () => props.mapPhotos,
  (newPhotos) => {
    if (!props.map) return
    const source = props.map.getSource('photos-dots') as GeoJSONSource | undefined
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
  <slot />
</template>
