<script setup lang="ts">
import MdiArrowTopRight from '~icons/mdi/arrow-top-right'
import { computed, onUnmounted, ref, shallowRef, watch } from 'vue'
import type { FullMediaItem } from '@/scripts/types/api/fullPhoto.ts'
import type { SharedMediaItem } from '@/scripts/types/api/album.ts'
import BaseMap from '@/vues/components/map/BaseMap.vue'
import EditLocationCard from '@/vues/components/viewer/components/EditLocationCard.vue'
import { useTheme } from 'vuetify/framework'
import { makeLocationString } from '@/scripts/utils.ts'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { Map as LibreMap, Marker } from 'maplibre-gl'

const props = defineProps<{
  mediaItem: FullMediaItem | SharedMediaItem
}>()
const theme = useTheme()
const settings = useSettingStore()
const authStore = useAuthStore()

const locationDialogOpen = ref(false)
const canEdit = computed(() => authStore.isAuthenticated && 'user_id' in props.mediaItem)

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
  <div class="map-info" v-if="mediaItem?.gps || canEdit">
    <div class="media-location-map" v-if="mediaItem?.gps">
      <base-map
        class="base-map-comp"
        :map-style="mapTheme === 'light' ? 'LIBERTY' : 'DARK'"
        :map-options="{
          center: { lat: mediaItem.gps.latitude, lon: mediaItem.gps.longitude },
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
          v-if="mediaItem?.gps"
          v-ripple
          :href="`https://www.google.com/maps/place/${mediaItem.gps.latitude},${mediaItem.gps.longitude}`"
          target="_blank"
          referrerpolicy="no-referrer"
        >
          <span v-if="mediaItem.gps.location">{{
            makeLocationString(mediaItem.gps.location, 3)
          }}</span>
          <v-icon size="15" class="ml-2 map-button-icon" :icon="MdiArrowTopRight" />
        </a>
        <span v-else class="no-location">No location</span>

        <v-btn
          v-if="canEdit"
          variant="plain"
          density="compact"
          rounded
          color="primary"
          class="edit-button"
          @click="locationDialogOpen = true"
        >
          {{ mediaItem?.gps ? 'Edit' : 'Add location' }}
        </v-btn>
      </v-sheet>
    </v-theme-provider>

    <v-dialog v-if="canEdit" max-width="560" persistent v-model="locationDialogOpen">
      <edit-location-card
        v-if="'user_id' in mediaItem"
        :media-item="mediaItem"
        @close-dialog="locationDialogOpen = false"
      />
    </v-dialog>
  </div>
</template>

<style scoped>
.map-info {
  border-radius: 20px;
  overflow: hidden;
}

.map-buttons {
  background-color: rgba(var(--v-theme-surface), 0.9);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-right: 6px;
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
  flex: 1;
  min-width: 0;
}

.no-location {
  color: rgba(var(--v-theme-on-surface-variant), 1);
  font-weight: 500;
  font-size: 13px;
  padding: 7px 20px;
  opacity: 0.7;
}

.edit-button {
  font-size: 14px;
}

.map-button-icon {
  font-weight: lighter;
  opacity: 0.8;
}

.media-location-map {
  --marker-dot-color: v-bind(bgColor);
  width: 100%;
  height: 300px;
  position: relative;
}

.base-map-comp {
  width: 100%;
  height: 100%;
}
</style>
