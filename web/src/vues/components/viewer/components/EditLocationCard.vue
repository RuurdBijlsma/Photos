<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { Map as LibreMap, Marker } from 'maplibre-gl'
import BaseMap from '@/vues/components/map/BaseMap.vue'
import type { FullMediaItem } from '@/scripts/types/api/fullPhoto.js'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.js'
import { useSettingStore } from '@/scripts/stores/settingsStore.js'
import { useTheme } from 'vuetify/framework'

const props = defineProps<{
  mediaItem: FullMediaItem
}>()

const emit = defineEmits(['closeDialog'])
const mediaItemStore = useMediaItemStore()
const settings = useSettingStore()
const theme = useTheme()

const latitude = ref<number | string>(props.mediaItem.gps?.latitude ?? '')
const longitude = ref<number | string>(props.mediaItem.gps?.longitude ?? '')
const saving = ref(false)
const showManualInput = ref(false)

const mapTheme = computed(() =>
  settings.lightPhotoViewerMap || !theme.current.value.dark ? 'light' : 'dark',
)
const mapStyle = computed(() => (mapTheme.value === 'light' ? 'LIBERTY' : 'DARK'))
const primaryColor = computed(() => String(theme.themes.value[mapTheme.value].colors.primary))
const bgColor = computed(() => String(theme.themes.value[mapTheme.value].colors['on-primary']))

const coordinates = computed(() => {
  const lat = Number(latitude.value)
  const lon = Number(longitude.value)
  const valid =
    latitude.value !== '' &&
    longitude.value !== '' &&
    Number.isFinite(lat) &&
    Number.isFinite(lon) &&
    Math.abs(lat) <= 90 &&
    Math.abs(lon) <= 180
  return { lat, lon, valid }
})

const mapOptions = computed(() => ({
  center: coordinates.value.valid
    ? { lat: coordinates.value.lat, lon: coordinates.value.lon }
    : { lat: 20, lon: 0 },
  zoom: props.mediaItem.gps ? 11 : 2,
  attributionControl: { compact: true },
}))

let mapInstance: LibreMap | null = null
let marker: Marker | null = null

function moveMarker() {
  if (!mapInstance || !coordinates.value.valid) return
  const position: [number, number] = [coordinates.value.lon, coordinates.value.lat]
  if (marker) {
    marker.setLngLat(position)
  } else {
    marker = new Marker({ color: primaryColor.value }).setLngLat(position).addTo(mapInstance)
  }
}

function handleMapLoad(loadedMap: LibreMap) {
  mapInstance = loadedMap
  moveMarker()
  loadedMap.on('click', (event) => {
    latitude.value = event.lngLat.lat
    longitude.value = event.lngLat.lng
  })
}

watch(coordinates, moveMarker)

onUnmounted(() => {
  marker?.remove()
  marker = null
})

async function save() {
  if (!coordinates.value.valid) return
  saving.value = true
  await mediaItemStore.updateMediaItem(props.mediaItem.id, {
    gps: { latitude: coordinates.value.lat, longitude: coordinates.value.lon },
  })
  saving.value = false
  emit('closeDialog')
}
</script>

<template>
  <v-card class="edit-location-card pa-6" color="surface-container" rounded="xl">
    <v-card-title class="text-center mb-6">
      {{ mediaItem.gps ? 'Edit' : 'Add' }} Location
    </v-card-title>

    <div class="map-picker">
      <base-map
        class="base-map"
        :map-style="mapStyle"
        :map-options="mapOptions"
        show-zoom-buttons
        @load="handleMapLoad"
      />
    </div>

    <v-expand-transition>
      <div v-if="showManualInput" class="coordinates">
        <v-text-field
          v-model.number="latitude"
          type="number"
          label="Latitude"
          variant="outlined"
          base-color="outline"
          color="primary"
          density="comfortable"
          rounded
          hide-details
        />
        <v-text-field
          v-model.number="longitude"
          type="number"
          label="Longitude"
          variant="outlined"
          base-color="outline"
          color="primary"
          density="comfortable"
          rounded
          hide-details
        />
      </div>
    </v-expand-transition>

    <v-card-actions class="mt-5">
      <p class="hint">
        <span v-if="coordinates.valid">
          {{ coordinates.lat.toFixed(5) }}, {{ coordinates.lon.toFixed(5) }}
        </span>
        <span v-else>Click the map to place the pin.</span>
        <v-btn
          variant="text"
          density="compact"
          size="small"
          class="manual-toggle"
          @click="showManualInput = !showManualInput"
        >
          {{ showManualInput ? 'Hide input' : 'Enter manually' }}
        </v-btn>
      </p>
      <v-spacer />
      <v-btn variant="text" rounded @click="emit('closeDialog')"> Cancel </v-btn>
      <v-btn
        variant="tonal"
        color="primary"
        rounded
        :disabled="!coordinates.valid"
        :loading="saving"
        @click="save"
      >
        Save
      </v-btn>
    </v-card-actions>
  </v-card>
</template>

<style scoped>
.edit-location-card {
  background-color: rgba(var(--v-theme-surface-container), 0.8) !important;
  backdrop-filter: saturate(250%) blur(12px) !important;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.map-picker {
  height: 300px;
  border-radius: 16px;
  overflow: hidden;
  --marker-dot-color: v-bind(bgColor);
}

.base-map {
  width: 100%;
  height: 100%;
}

.hint {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  opacity: 0.7;
  margin-top: 10px;
  min-height: 32px;
}

.manual-toggle {
  font-size: 12px;
  margin-left: 10px;
}

.coordinates {
  display: flex;
  gap: 12px;
  margin-top: 30px;
}
</style>
