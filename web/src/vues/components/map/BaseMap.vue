<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import {
  Map as LibreMap,
  type MapOptions,
  NavigationControl,
  setWorkerUrl,
  type StyleSpecification,
} from 'maplibre-gl'
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url'
import 'maplibre-gl/dist/maplibre-gl.css'

setWorkerUrl(workerUrl)

type MapOptionsWithoutContainer = Omit<MapOptions, 'container' | 'style'>

const props = withDefaults(
  defineProps<{
    mapStyle?: StyleName
    mapOptions: MapOptionsWithoutContainer
    showCompass?: boolean
    showZoomButtons?: boolean
  }>(),
  {
    mapStyle: 'LIBERTY',
    showCompass: false,
    showZoomButtons: false,
    mapOptions: () => ({
      center: { lon: 0, lat: 0 },
      zoom: 2,
      attributionControl: {
        compact: true,
      },
    }),
  },
)

const emit = defineEmits(['load', 'style-load'])

const mapContainer = ref<HTMLElement | null>(null)
let map: null | LibreMap = null

const styles = {
  SATELLITE: {
    version: 8,
    sources: {
      satellite: {
        type: 'raster',
        tiles: [
          'https://mt0.google.com/vt/lyrs=s&x={x}&y={y}&z={z}',
          'https://mt1.google.com/vt/lyrs=s&x={x}&y={y}&z={z}',
          'https://mt2.google.com/vt/lyrs=s&x={x}&y={y}&z={z}',
          'https://mt3.google.com/vt/lyrs=s&x={x}&y={y}&z={z}',
        ],
        tileSize: 256,
      },
      // ── Elevation data ──────────────────────────
      terrarium: {
        type: 'raster-dem',
        tiles: ['https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png'],
        tileSize: 256,
        encoding: 'terrarium',
      },
    },
    layers: [{ id: 'satellite', type: 'raster', source: 'satellite' }],
    // ── Enable 3D terrain from elevation source ──
    terrain: { source: 'terrarium', exaggeration: 1.5 },
  },
  LIBERTY: 'https://tiles.openfreemap.org/styles/liberty',
  TERRAIN: 'https://tiles.stadiamaps.com/styles/stamen_terrain.json',
  WATERCOLOR: 'https://tiles.stadiamaps.com/styles/stamen_watercolor.json',
  DARK: 'https://tiles.stadiamaps.com/styles/alidade_smooth_dark.json',
  DARK_COLORFUL: 'https://tiles.versatiles.org/assets/styles/eclipse/style.json',
}
export type StyleName = keyof typeof styles

onMounted(() => {
  if (!mapContainer.value) return
  map = new LibreMap({
    ...props.mapOptions,
    container: mapContainer.value,
    style: styles[props.mapStyle] as unknown as StyleSpecification,
  })

  if (props.showCompass || props.showZoomButtons) {
    map.addControl(
      new NavigationControl({
        showCompass: props.showCompass,
        showZoom: props.showZoomButtons,
        visualizePitch: true,
      }),
      'bottom-right',
    )
  }

  map.on('load', () => {
    emit('load', map)
  })

  map.on('style.load', () => {
    // ── Add 3D Terrain for the TERRAIN style ──
    if (map && props.mapStyle === 'TERRAIN') {
      if (!map.getSource('terrarium')) {
        map.addSource('terrarium', {
          type: 'raster-dem',
          tiles: ['https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png'],
          tileSize: 256,
          encoding: 'terrarium',
        })
      }
      map.setTerrain({ source: 'terrarium', exaggeration: 1.5 })
    }

    emit('style-load', map)
  })
})

onUnmounted(() => {
  if (map) map.remove()
})

watch(
  () => props.mapOptions.center,
  () => {
    if (map === null || props.mapOptions.center === undefined) return
    map.setCenter(props.mapOptions.center)
  },
)

watch(
  () => props.mapOptions.zoom,
  () => {
    if (map === null || props.mapOptions.zoom === undefined) return
    map.setZoom(props.mapOptions.zoom)
  },
)

watch(
  () => props.mapStyle,
  (newStyle) => {
    if (map === null || !newStyle) return
    map.setStyle(styles[newStyle] as unknown as StyleSpecification)
  },
)
</script>

<template>
  <div ref="mapContainer"></div>
</template>

<style>
/* --- Custom Rounded Compass with Red North Arrow --- */

.maplibregl-ctrl-group:has(.maplibregl-ctrl-compass) {
  border-radius: 50% !important;
  overflow: hidden;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25) !important;
  background-color: rgba(255, 255, 255, 0.9) !important;
  backdrop-filter: blur(8px) !important;
  border: 1px solid rgba(0, 0, 0, 0.08) !important;
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease;
}

.maplibregl-ctrl-group:has(.maplibregl-ctrl-compass):hover {
  transform: scale(1.02);
  background-color: #fff !important;
}

.maplibregl-ctrl-group button.maplibregl-ctrl-compass {
  width: 42px !important;
  height: 42px !important;
  border-radius: 50% !important;
  display: flex !important;
  align-items: center;
  justify-content: center;
}

/* Make the needle larger & always visible */
.maplibregl-ctrl button.maplibregl-ctrl-compass .maplibregl-ctrl-icon {
  width: 32px !important;
  height: 32px !important;
  background-size: contain !important;
  background-position: center !important;
  background-repeat: no-repeat !important;

  /* Custom 2-color SVG needle: Red for North (#ef4444) and Slate Gray for South (#94a3b8) */
  background-image: url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 29 29'%3E%3Cpath d='M10.5 14.5 L14.5 1.5 L18.5 14.5 Z' fill='%23ef4444'/%3E%3Cpath d='M10.5 14.5 L14.5 27.5 L18.5 14.5 Z' fill='%2394a3b8'/%3E%3C/svg%3E") !important;
}

/* Keep the compass button visible even when facing directly North (bearing = 0) */
.maplibregl-ctrl-compass {
  display: flex !important;
}
</style>
