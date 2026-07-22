<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import maplibregl from 'maplibre-gl'
import BaseMap, { type StyleName } from '@/vues/components/map/BaseMap.vue'
import type { LocationMediaItem } from '@/scripts/types/generated/timeline.ts'
import { useStorage } from '@vueuse/core'

const props = defineProps<{
  items: LocationMediaItem[]
}>()

const mapInstance = ref<maplibregl.Map | null>(null)

// Filters out items without physical coordinates
const geoItems = computed(() => {
  return props.items.filter(
    (item) => typeof item.latitude === 'number' && typeof item.longitude === 'number',
  )
})

// Determines if we have any valid coordinates to plot
const hasCoordinates = computed(() => geoItems.value.length > 0)

// Standardizes calculations for coordinates bounds
const getBounds = () => {
  if (geoItems.value.length === 0) return null
  const first = geoItems.value[0]!
  const bounds = new maplibregl.LngLatBounds(
    [first.longitude!, first.latitude!],
    [first.longitude!, first.latitude!],
  )
  for (const item of geoItems.value) {
    bounds.extend([item.longitude!, item.latitude!])
  }
  return bounds
}

const mapStyle = useStorage<StyleName>('mapCurrentStyle', 'LIBERTY')

// Map initialization configuration
const mapOptions = computed(() => {
  const bounds = getBounds()
  if (!bounds) {
    return {
      center: { lon: 0, lat: 0 },
      zoom: 2,
      attributionControl: { compact: true },
      scrollZoom: false, // Disables scroll zoom upon map initialization
    }
  }
  return {
    bounds,
    fitBoundsOptions: {
      padding: 40,
      maxZoom: 15,
    },
    attributionControl: { compact: true },
    scrollZoom: false, // Disables scroll zoom upon map initialization
  }
})

function handleMapLoad(map: maplibregl.Map) {
  mapInstance.value = map

  // Add standard built-in +/- zoom controls in the bottom right
  map.addControl(
    new maplibregl.NavigationControl({
      showCompass: false,
      showZoom: true,
    }),
    'bottom-right',
  )

  setupMapResources()
}

function handleStyleLoad() {
  setupMapResources()
}

function setupMapResources() {
  const map = mapInstance.value
  if (!map) return

  // Safe layer and source teardown to avoid duplicates on style switches
  if (map.getLayer('media-points')) map.removeLayer('media-points')
  if (map.getSource('media-source')) map.removeSource('media-source')

  const geojson: GeoJSON.FeatureCollection<GeoJSON.Point> = {
    type: 'FeatureCollection',
    features: geoItems.value.map((item) => ({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: [item.longitude!, item.latitude!],
      },
      properties: {
        id: item.id,
      },
    })),
  }

  map.addSource('media-source', {
    type: 'geojson',
    data: geojson,
  })

  // Add GPU-rendered point markers to the map
  map.addLayer({
    id: 'media-points',
    type: 'circle',
    source: 'media-source',
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], 1, 4, 10, 6, 15, 9],
      'circle-color': 'rgb(80, 30, 120)', // Subtle deep purple point color
      'circle-stroke-color': '#ffffff',
      'circle-stroke-width': 1.5,
      'circle-opacity': 0.85,
      'circle-stroke-opacity': 0.9,
    },
  })
}

// React to dynamic items changes
watch(
  geoItems,
  () => {
    const map = mapInstance.value
    if (!map) return

    setupMapResources()

    const bounds = getBounds()
    if (bounds) {
      map.fitBounds(bounds, {
        padding: 40,
        maxZoom: 15,
        duration: 800,
      })
    }
  },
  { deep: true },
)
</script>

<template>
  <div v-if="hasCoordinates" class="location-media-map-container">
    <base-map
      class="location-media-map"
      :map-style="mapStyle"
      :map-options="mapOptions"
      @load="handleMapLoad"
      @style-load="handleStyleLoad"
    />
  </div>
</template>

<style scoped>
.location-media-map-container {
  width: 100%;
  height: 480px;
  overflow: hidden;
  border-radius: 50px;
  border: 15px solid rgb(var(--v-theme-surface-container));
  margin-top: 0;
}

.location-media-map {
  width: 100%;
  height: 100%;
}
</style>
