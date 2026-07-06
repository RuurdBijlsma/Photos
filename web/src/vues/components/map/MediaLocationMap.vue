<script setup lang="ts">
import { ref, onUnmounted, watch, computed } from 'vue'
import maplibregl from 'maplibre-gl'
import type { FullMediaItem } from '@/scripts/types/api/fullPhoto.ts'
import type { SharedMediaItem } from '@/scripts/types/api/album.ts'
import BaseMap from '@/vues/components/map/BaseMap.vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { useTheme } from 'vuetify/framework'
import { makeLocationString } from '@/scripts/utils.ts'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'

const props = defineProps<{
  mediaItem: FullMediaItem | SharedMediaItem
}>()
const theme = useTheme()
const settings = useSettingStore()

const mapInstance = ref<maplibregl.Map | null>(null)
let markerInstance: maplibregl.Marker | null = null

function getThumbnailUrl() {
  if (!props.mediaItem) return ''
  return mediaItemService.getPhotoThumbnail(
    props.mediaItem.id,
    144,
    !props.mediaItem.has_thumbnails,
  )
}

function handleMapLoad(loadedMap: maplibregl.Map) {
  mapInstance.value = loadedMap
  updateMarker()
}

function updateMarker() {
  if (!mapInstance.value || !props.mediaItem?.gps) return

  const lat = props.mediaItem.gps.latitude
  const lon = props.mediaItem.gps.longitude
  const thumbUrl = getThumbnailUrl()

  if (!markerInstance) {
    // Create custom pin element matching MapView style
    const el = document.createElement('div')
    el.className = 'media-map-pin'

    const circle = document.createElement('div')
    circle.className = 'marker-circle'
    circle.style.backgroundImage = `url(${thumbUrl})`

    const triangle = document.createElement('div')
    triangle.className = 'marker-triangle'

    el.appendChild(circle)
    el.appendChild(triangle)

    // Anchor to bottom so the tip of the triangle points exactly to the coordinate
    markerInstance = new maplibregl.Marker({
      element: el,
      anchor: 'bottom',
    })
      .setLngLat([lon, lat])
      .addTo(mapInstance.value as unknown as maplibregl.Map)
  } else {
    markerInstance.setLngLat([lon, lat])
    const circle = markerInstance.getElement().querySelector('.marker-circle') as HTMLElement
    if (circle) {
      circle.style.backgroundImage = `url(${thumbUrl})`
    }
  }

  // Focus the map view on the pin
  mapInstance.value.setCenter([lon, lat])
}

// Watch media item and GPS coordinate changes to update the pin's position and image
watch(
  () => [props.mediaItem?.id, props.mediaItem?.gps?.latitude, props.mediaItem?.gps?.longitude],
  () => {
    updateMarker()
  },
)

onUnmounted(() => {
  if (markerInstance) {
    markerInstance.remove()
    markerInstance = null
  }
})

console.log('thtthth', theme.themes.value, theme)

const mapMarkerColor = computed(() => {
  const isLight = settings.lightPhotoViewerMap || !theme.current.value.dark
  return theme.themes.value[isLight ? 'light' : 'dark'].colors['on-surface']
})

watch(
  mapMarkerColor,
  () => {
    console.log(mapMarkerColor.value)
  },
  { immediate: true },
)
</script>

<template>
  <div class="map-info" v-if="mediaItem?.gps">
    <div class="media-location-map">
      <base-map
        v-if="props.mediaItem?.gps"
        class="base-map-comp"
        :map-style="settings.lightPhotoViewerMap || !theme.current.value.dark ? 'LIBERTY' : 'DARK'"
        :map-options="{
          center: { lat: props.mediaItem.gps.latitude, lon: props.mediaItem.gps.longitude },
          zoom: 9,
          attributionControl: {
            compact: true,
          },
        }"
        @load="handleMapLoad"
      />
    </div>
    <v-theme-provider
      with-background
      :theme="settings.lightPhotoViewerMap || !theme.current.value.dark ? 'light' : 'dark'"
    >
      <v-sheet class="map-buttons">
        <a
          v-ripple
          :href="`https://www.google.com/maps/place/${mediaItem.gps.latitude},${mediaItem.gps.longitude}`"
          target="_blank"
          referrerpolicy="no-referrer"
        >
          <span v-if="mediaItem.gps.location">{{
            makeLocationString(mediaItem.gps.location, 3)
          }}</span>
          <v-icon size="15" class="ml-2 map-button-icon" icon="mdi-arrow-top-right" />
        </a>
      </v-sheet>
    </v-theme-provider>
  </div>
</template>

<style scoped>
.map-info {
  border-radius: 20px;
  overflow: hidden;
}

.map-buttons {
  background-color: rgba(var(--v-theme-surface), 0.9);
}

.map-buttons a {
  color: rgba(var(--v-theme-on-surface-variant), 1);
  text-decoration: none;
  display: flex;
  align-items: center;
  user-select: none;
  font-weight: 500;
  font-size: 13px;
  padding: 7px 20px;
}

.map-button-icon {
  font-weight: lighter;
  opacity: 0.8;
}

.media-location-map {
  width: 100%;
  height: 300px;
  position: relative;
}

.base-map-comp {
  width: 100%;
  height: 100%;
}
</style>

<style>
/* Style for custom DOM marker */
.media-map-pin {
  width: 52px;
  height: 60px;
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.4));
  pointer-events: none;
}

.marker-circle {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  border: 2px solid v-bind(mapMarkerColor);
  background-color: rgba(20, 20, 24, 0.65);
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  box-sizing: border-box;
  box-shadow: 0 0 0 4px color-mix(in srgb, v-bind(mapMarkerColor) 40%, transparent);
}

.marker-triangle {
  width: 0;
  height: 0;
  border-left: 6px solid transparent;
  border-right: 6px solid transparent;
  border-top: 8px solid v-bind(mapMarkerColor);
  margin-top: -1px;
}
</style>
