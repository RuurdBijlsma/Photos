<script setup lang="ts">
import type { FullMediaItem, MediaItemAlbumRef } from '@/scripts/types/api/fullPhoto.js'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import { useDialogStore } from '@/scripts/stores/dialogStore.js'
import { useSettingStore } from '@/scripts/stores/settingsStore.js'
import { computed, defineAsyncComponent, ref, watch } from 'vue'
import { DAYS, MONTHS } from '@/scripts/constants.js'
import MediaWeatherInfo from '@/vues/components/viewer/components/MediaWeatherInfo.vue'
import { caps, prettyBytes, toHms } from '@/scripts/utils.js'
import EditDateTimeCard from '@/vues/components/viewer/components/EditDateTimeCard.vue'
import { useAuthStore } from '@/scripts/stores/authStore.js'
import type { SharedMediaItem } from '@/scripts/types/api/album.js'
import { useRoute } from 'vue-router'
import { useTheme } from 'vuetify/framework'
import { getCodecInfo } from '@/scripts/codecUtils.js'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.js'

const MediaLocationMap = defineAsyncComponent(
  () => import('@/vues/components/map/MediaLocationMap.vue'),
)

const props = defineProps<{
  mediaItem?: FullMediaItem | SharedMediaItem
  albums?: MediaItemAlbumRef[]
}>()

const emit = defineEmits(['closeDateTime', 'openDateTime'])

const dialogs = useDialogStore()
const settings = useSettingStore()
const authStore = useAuthStore()
const mediaItemStore = useMediaItemStore()
const route = useRoute()
const theme = useTheme()

const currentAlbumId = computed(() => route.params.albumId as string | undefined)

const showAlbums = computed(
  () => authStore.isAuthenticated && props.albums !== undefined && props.albums.length > 0,
)

const dateTimeDialogOpen = ref(false)

const takenAtDate = computed(() => {
  if (!props.mediaItem?.taken_at_local) return null
  return new Date(props.mediaItem?.taken_at_local)
})

const takenAtDateMyTz = computed(() => {
  if (!props.mediaItem?.taken_at_utc) return null
  return new Date(props.mediaItem?.taken_at_utc)
})

function dateComponents(date: Date) {
  const day = DAYS[date.getDay()]
  const dateString = `${date.getDate()}-${MONTHS[date.getMonth()].substring(0, 3)}-${date.getFullYear().toString().substring(2)}`
  const time = `${date.getHours()}:${date.getMinutes()}`

  return { day, dateString, time }
}

function fullDateString(date: Date) {
  const components = dateComponents(date)
  return `${components.day} • ${components.dateString} • ${components.time}`
}

async function editCaption() {
  if (!props.mediaItem?.id) return
  const oldCaption = props.mediaItem?.user_caption ?? ''
  const newCaption = await dialogs.prompt({
    title: 'Edit Caption',
    defaultValue: oldCaption,
    confirmText: 'Update',
    attach: true,
  })
  if (newCaption === '')
    return await mediaItemStore.updateMediaItem(props.mediaItem.id, { userCaption: null })
  if (!newCaption) return
  await mediaItemStore.updateMediaItem(props.mediaItem.id, { userCaption: newCaption })
}

watch(dateTimeDialogOpen, () => {
  if (dateTimeDialogOpen.value) {
    dialogs.customVisible = true
    emit('openDateTime')
  } else {
    dialogs.customVisible = false
    emit('closeDateTime')
  }
})

// Camera info computeds
const cameraDisplayName = computed(() => {
  const cs = props.mediaItem?.camera_settings
  if (!cs) return null
  const make = cs.camera_make?.trim() ?? cs.lens_make?.trim()
  const model = cs.camera_model?.trim()
  if (!make && !model) return null
  if (!make) return model!
  if (!model) return make
  // Avoid duplication like "Canon Canon EOS R5"
  if (model.toLowerCase().startsWith(make.toLowerCase())) return model
  return `${make} ${model}`
})

const fileTypeLabel = computed(() => {
  const mime = props.mediaItem?.media_features?.mime_type
  if (!mime) return null
  const sub = mime.split('/')[1]
  if (!sub) return null
  const map: Record<string, string> = {
    quicktime: 'MOV',
    'x-msvideo': 'AVI',
    'x-matroska': 'MKV',
  }
  return map[sub] ?? sub.toUpperCase()
})

const featureBadges = computed(() => {
  const features = props.mediaItem?.media_features
  if (!features) return []
  const badges: string[] = []
  if (features.is_hdr) badges.push('HDR')
  if (features.is_nightsight) badges.push('Night')
  if (features.is_motion_photo) badges.push('Motion')
  if (features.is_burst) badges.push('Burst')
  if (features.is_timelapse) badges.push('Timelapse')
  return badges
})

const lensDisplayName = computed(() => {
  const cs = props.mediaItem?.camera_settings
  if (!cs) return null
  const lensModel = cs.lens_model?.trim()
  const lensMake = cs.lens_make?.trim()
  const cameraMake = cs.camera_make?.trim()
  const cameraModel = cs.camera_model?.trim()
  if (!lensMake && !lensModel) return null
  let result
  if (!lensMake) {
    result = lensModel!
  } else if (!lensModel) {
    result = lensMake!
  } else {
    if (lensModel.startsWith(lensMake)) {
      result = lensModel!
    } else {
      result = `${lensMake} ${lensModel!}`
    }
  }

  if (cameraMake) {
    if (result.startsWith(cameraMake) && result.length !== cameraMake.length) {
      result = result.replace(cameraMake, '').trim()
    }
  }
  if (cameraModel) {
    if (result.startsWith(cameraModel) && result.length !== cameraModel.length) {
      result = result.replace(cameraModel, '').trim()
    }
  }

  return caps(result)
})

const mediaSpecsLine = computed(() => {
  const item = props.mediaItem
  if (!item) return null
  const parts: string[] = []

  if (!item.is_video && item.width && item.height) {
    const mp = (item.width * item.height) / 1_000_000
    parts.push(`${Math.round(mp)} MP`)
  }

  if (item.width && item.height) {
    parts.push(`${item.width} × ${item.height}`)
  }

  if (item.is_video && item.duration_ms) {
    parts.push(toHms(item.duration_ms / 1000))
  }

  const bytes = item.media_features?.size_bytes
  if (bytes) {
    parts.push(prettyBytes(bytes))
  }

  return parts.length > 0 ? parts.join(' · ') : null
})

const exposureItems = computed(() => {
  const cs = props.mediaItem?.camera_settings
  if (!cs) return []
  const items: { label: string; tooltip?: string | undefined }[] = []

  if (cs.iso != null) items.push({ label: `ISO ${cs.iso}` })
  if (cs.focal_length != null || cs.focal_length_in_35mm != null) {
    if (cs.focal_length === null) {
      items.push({
        label: `${cs.focal_length_in_35mm} mm eq`,
        tooltip: '35mm-equivalent focal length',
      })
    } else if (cs.focal_length_in_35mm === null) {
      items.push({
        label: `${cs.focal_length} mm`,
        tooltip: 'Actual focal length (35mm equivalent unavailable)',
      })
    } else {
      items.push({
        label: `${cs.focal_length_in_35mm} mm eq`,
        tooltip: `35mm-equivalent focal length • Actual: ${cs.focal_length} mm`,
      })
    }
  }
  if (cs.exposure_compensation != null)
    items.push({ label: `${Math.round(cs.exposure_compensation * 100) / 100} ev` })
  if (cs.aperture != null) items.push({ label: `ƒ${cs.aperture}` })

  if (cs.exposure_time != null) {
    if (cs.exposure_time >= 0.5) {
      items.push({ label: `${Math.round(cs.exposure_time * 100) / 100} s` })
    } else {
      const denom = Math.round(1 / cs.exposure_time)
      items.push({ label: `1/${denom} s` })
    }
  }

  // Video FPS
  if (props.mediaItem?.is_video) {
    const fps =
      props.mediaItem.media_features.video_fps ?? props.mediaItem.media_features.capture_fps
    if (fps) {
      items.push({ label: `${Math.round(fps * 10) / 10} fps` })
    }
  }

  // Video Bitrate
  const item = props.mediaItem
  const bytes = item?.media_features?.size_bytes
  const durationMs = item?.duration_ms
  const isVideo = item?.is_video
  if (bytes && durationMs && isVideo) {
    const bytesPerSecond = Math.round(bytes / (durationMs / 1000) / 1000) * 1000
    const bitRate = prettyBytes(bytesPerSecond) + '/s'
    items.push({ label: bitRate })
  }

  if (isVideo && item?.media_features?.compressor_id) {
    const codecInfo = getCodecInfo(item.media_features.compressor_id)
    if (codecInfo) {
      items.push({ label: codecInfo?.friendlyName })
    }
  }

  return items
})

const showCameraSection = computed(() => {
  return !!(
    cameraDisplayName.value ||
    fileTypeLabel.value ||
    mediaSpecsLine.value ||
    exposureItems.value.length > 0
  )
})
</script>

<template>
  <div
    class="info-panel"
    :class="{ 'backdrop-blur': settings.useBackdropBlur, light: !theme.current.value.dark }"
  >
    <h2 class="info-title">Info</h2>
    <div class="info-loading" v-if="mediaItem === undefined">
      <v-progress-circular indeterminate size="50" />
      <h2>Loading info...</h2>
    </div>
    <template v-else>
      <div class="caption">
        <div class="user-caption">
          <div class="user-caption-holder" v-if="mediaItem.user_caption">
            <p class="user-caption-title">Caption</p>
            <p class="user-caption-caption">{{ mediaItem.user_caption }}</p>
          </div>
          <p class="no-caption" v-else>No caption</p>
          <v-btn
            v-if="authStore.isAuthenticated"
            variant="plain"
            @click="editCaption"
            density="compact"
            rounded
            color="primary"
            class="edit-button"
          >
            Edit
          </v-btn>
        </div>
      </div>
      <v-divider class="mt-2 mb-2" />
      <div class="date-time-weather-filename">
        <div class="date-time-container">
          <p
            v-tooltip="{
              disabled: takenAtDateMyTz === null,
              text: 'In your timezone • ' + fullDateString(takenAtDateMyTz ?? new Date()),
              location: 'bottom',
              attach: true,
            }"
            v-if="takenAtDate"
            class="date-time"
          >
            <span>{{ dateComponents(takenAtDate).day }}</span>
            <span>•</span>
            <span>{{ dateComponents(takenAtDate).dateString }}</span>
            <span>•</span>
            <span>{{ dateComponents(takenAtDate).time }}</span>
          </p>
          <v-dialog max-width="420" persistent v-model="dateTimeDialogOpen">
            <template v-slot:activator="{ props: activatorProps }">
              <v-btn
                variant="plain"
                density="compact"
                rounded
                color="primary"
                class="edit-button"
                v-if="authStore.isAuthenticated"
                v-bind="activatorProps"
              >
                Edit
              </v-btn>
            </template>

            <template v-slot:default="{ isActive }">
              <edit-date-time-card
                v-if="'user_id' in mediaItem"
                :media-item="mediaItem"
                @close-dialog="isActive.value = false"
              />
            </template>
          </v-dialog>
        </div>
        <media-weather-info
          :weather-info="mediaItem?.weather"
          v-if="
            mediaItem?.weather && (mediaItem.weather.temperature || mediaItem.weather.condition)
          "
        />
        <p class="filename">
          <v-icon icon="mdi-cloud-check-outline" class="mr-3" /><span>{{
            mediaItem?.filename.split('.')[0]
          }}</span>
        </p>
      </div>
      <section v-if="showAlbums" class="albums-section">
        <p class="section-label">Albums</p>
        <router-link
          v-for="album in albums"
          :key="album.id"
          :to="`/album/${album.id}`"
          class="album-row"
          :class="{ 'current-album': currentAlbumId === album.id }"
          v-ripple
        >
          <v-avatar size="30" rounded class="album-avatar" color="surface-container-high">
            <thumbnail-img
              v-if="album.thumbnail_id"
              :media-item-id="album.thumbnail_id"
              :height="144"
              cover
            />
            <v-icon v-else icon="mdi-image-album" size="18" class="album-fallback-icon" />
          </v-avatar>
          <span class="album-text">
            <span
              class="album-name"
              v-tooltip:bottom="{ text: album.name, disabled: album.name.length <= 28 }"
            >
              {{ album.name || 'Unnamed' }}
            </span>
            <span class="album-count">
              {{ album.media_count.toLocaleString() }}
              item{{ album.media_count === 1 ? '' : 's' }}
            </span>
          </span>
          <v-icon icon="mdi-chevron-right" size="16" class="album-chevron" />
        </router-link>
      </section>
      <div class="camera-info" v-if="showCameraSection">
        <div
          class="camera-header"
          v-if="cameraDisplayName || fileTypeLabel || featureBadges.length > 0"
        >
          <span class="camera-model" v-if="cameraDisplayName">{{ cameraDisplayName }}</span>
          <span class="lens-info no-lens" v-else>No camera info</span>
          <div class="camera-badges">
            <span v-for="badge in featureBadges" :key="badge" class="feature-badge">{{
              badge
            }}</span>
            <span v-if="fileTypeLabel" class="file-type-badge">{{ fileTypeLabel }}</span>
          </div>
        </div>
        <p class="lens-info" v-if="lensDisplayName">{{ lensDisplayName }}</p>
        <p class="lens-info no-lens" v-else-if="cameraDisplayName">No lens information</p>
        <p class="media-specs" v-if="mediaSpecsLine">{{ mediaSpecsLine }}</p>
        <div class="exposure-row" v-if="exposureItems.length > 0">
          <span
            v-for="(item, i) in exposureItems"
            :key="i"
            class="exposure-cell"
            v-tooltip="{
              location: 'bottom',
              text: item.tooltip,
              disabled: !item.tooltip,
            }"
            >{{ item.label }}</span
          >
        </div>
      </div>
      <media-location-map class="map-info" :media-item="mediaItem" />
    </template>
  </div>
</template>

<style scoped>
.info-panel {
  margin-top: 5px;
  width: 400px;
  border-radius: 30px;
  font-size: 15px;
  background-color: rgba(var(--v-theme-background), 0.8);
  color: rgb(var(--v-theme-on-background));
  position: relative;
}

.info-panel.light {
  background-color: rgba(var(--v-theme-background), 0.8);
}

.backdrop-blur {
  backdrop-filter: blur(15px) saturate(150%) brightness(90%) contrast(110%);
  background-color: rgba(var(--v-theme-background), 0.6);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.info-title {
  padding-left: 30px;
  padding-top: 10px;
  font-weight: 400;
  font-size: 20px;
  opacity: 0.9;
}

.info-loading {
  width: 100%;
  height: 500px;
  place-items: center;
  place-content: center;
  display: flex;
  flex-direction: column;
}

.info-loading h2 {
  font-size: 20px;
  font-weight: 400;
  opacity: 0.7;
  margin-top: 20px;
}

.caption {
  padding: 5px 30px;
}

.user-caption {
  display: flex;
  align-items: center;
  font-size: 13px;
  justify-content: space-between;
}

.user-caption-holder {
  display: flex;
  flex-direction: column;
}

.user-caption-title {
  opacity: 0.7;
  font-size: 13px;
}

.user-caption-caption {
  font-size: 16px;
}

.user-caption-holder p {
  margin: 0;
}

.no-caption {
  opacity: 0.7;
}

.edit-button {
  font-size: 14px;
}

.date-time-weather-filename {
  padding: 5px 30px;
}

.date-time-container {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.date-time {
  display: flex;
  gap: 7px;
  font-size: 14px;
  font-weight: 500;
}

.filename {
  font-size: 14px;
  font-weight: 500;
  opacity: 0.7;
  display: flex;
  align-items: center;
}

.albums-section {
  padding: 4px 22px 8px;
}

.section-label {
  font-size: 11px;
  font-weight: 400;
  letter-spacing: 0.06em;
  margin: 0 8px 6px;
  color: rgb(var(--v-theme-on-surface-variant));
  user-select: none;
}

.album-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  margin-bottom: 2px;
  border-radius: 10px;
  text-decoration: none;
  color: inherit;
  transition: background-color 0.15s ease;
}

.album-row.current-album {
  background-color: rgba(var(--v-theme-on-surface), 0.1);
}

.album-row:hover {
  background-color: rgba(var(--v-theme-on-surface), 0.15);
}

.album-row.current-album:hover {
  background-color: rgba(var(--v-theme-on-surface), 0.2);
}

.album-row:active {
  background-color: rgba(var(--v-theme-on-surface), 0.1);
}

.album-avatar {
  flex-shrink: 0;
}

.album-fallback-icon {
  opacity: 0.65;
}

.album-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.album-name {
  font-size: 14px;
  font-weight: 500;
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.album-count {
  font-size: 12px;
  opacity: 0.55;
  line-height: 1.2;
}

.album-chevron {
  flex-shrink: 0;
  opacity: 0.35;
}

.map-info {
  margin: 10px;
}

.camera-info {
  margin: 8px 20px;
  margin-bottom: 15px;
  padding: 12px 14px;
  border-radius: 14px;
  background-color: rgba(var(--v-theme-on-surface), 0.06);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.camera-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.camera-model {
  font-size: 15px;
  font-weight: 600;
}

.camera-badges {
  display: flex;
  gap: 5px;
  align-items: center;
}

.file-type-badge {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  padding: 2px 8px;
  border-radius: 6px;
  background-color: rgba(var(--v-theme-on-surface), 0.1);
}

.feature-badge {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  padding: 2px 7px;
  border-radius: 6px;
  background-color: rgba(var(--v-theme-primary), 0.15);
  color: rgb(var(--v-theme-primary));
}

.lens-info {
  font-size: 13px;
  font-style: italic;
  opacity: 0.7;
  margin: 0;
  display: flex;
  gap: 5px;
}

.lens-info.no-lens {
  opacity: 0.45;
}

.media-specs {
  font-size: 13px;
  opacity: 0.8;
  margin: 2px 0 0;
}

.exposure-row {
  display: flex;
  gap: 0;
  margin-top: 4px;
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  padding-top: 8px;
}

.exposure-cell {
  font-size: 12px;
  font-weight: 500;
  opacity: 0.75;
  padding: 0 10px;
  border-right: 1px solid rgba(var(--v-theme-on-surface), 0.15);
  white-space: nowrap;
}

.exposure-cell:first-child {
  padding-left: 0;
}

.exposure-cell:last-child {
  border-right: none;
}
</style>
