<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import maplibregl, { type MapOptions } from 'maplibre-gl'
import 'maplibre-gl/dist/maplibre-gl.css'

type MapOptionsWithoutContainer = Omit<MapOptions, 'container' | 'style'>

const props = withDefaults(
  defineProps<{
    mapStyle?: StyleName
    mapOptions: MapOptionsWithoutContainer
  }>(),
  {
    mapStyle: 'LIBERTY',
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
let map: null | maplibregl.Map = null

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
  map = new maplibregl.Map({
    ...props.mapOptions,
    container: mapContainer.value,
    style: styles[props.mapStyle] as unknown as maplibregl.StyleSpecification,
  })

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
    map.setStyle(styles[newStyle] as unknown as maplibregl.StyleSpecification)
  },
)
</script>

<template>
  <div ref="mapContainer"></div>
</template>
