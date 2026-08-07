<script setup lang="ts">
import { mdiArrowTopRight } from '@mdi/js'
import { computed, onUnmounted, shallowRef, watch } from 'vue'
import type { FullMediaItem } from '@/scripts/types/api/fullPhoto.ts'
import type { SharedMediaItem } from '@/scripts/types/api/album.ts'
import BaseMap from '@/vues/components/map/BaseMap.vue'
import { useTheme } from 'vuetify/framework'
import { makeLocationString } from '@/scripts/utils.ts'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { Map as LibreMap, Marker } from 'maplibre-gl'

const props = defineProps<{
  mediaItem: FullMediaItem | SharedMediaItem
}>()
const theme = useTheme()
const settings = useSettingStore()

const mapInstance = shallowRef<LibreMap | null>(null)
let markerInstance: Marker | null = null

const mapTheme = computed(() =>
  settings.lightPhotoViewerMap || !theme.current.value.dark ? 'light' : 'dark',
)
const primaryColor = computed(() => String(theme.themes.value[mapTheme.value].colors.primary))
const bgColor = computed(() => String(theme.themes.value[mapTheme.value].colors['on-primary']))

function handleMapLoad(loadedMap: LibreMap) {
  mapInstance.value = loadedMap
  updateMarker()
}

function updateMarker() {
  if (!mapInstance.value || !props.mediaItem?.gps) return

  const lat = props.mediaItem.gps.latitude
  const lon = props.mediaItem.gps.longitude

  if (!markerInstance) {
    markerInstance = new Marker({
      color: primaryColor.value,
    })
      .setLngLat([lon, lat])
      .addTo(mapInstance.value as LibreMap)
  } else {
    markerInstance.setLngLat([lon, lat])
  }

  // Focus the map view on the pin
  mapInstance.value.setCenter([lon, lat])
}

watch(
  [
    () => props.mediaItem?.id,
    () => props.mediaItem?.gps?.latitude,
    () => props.mediaItem?.gps?.longitude,
    primaryColor,
  ],
  () => {
    if (markerInstance) {
      markerInstance.remove()
      markerInstance = null
    }
    updateMarker()
  },
)

onUnmounted(() => {
  if (markerInstance) {
    markerInstance.remove()
    markerInstance = null
  }
})
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
    <v-theme-provider with-background :theme="mapTheme">
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
          <v-icon size="15" class="ml-2 map-button-icon" :icon="mdiArrowTopRight" />
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

/* Custom styling for MapLibre GL default SVG marker */
:deep(.maplibregl-marker circle) {
  fill: v-bind(bgColor) !important;
}

:deep(.maplibregl-marker svg) {
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.35));
}
</style>
