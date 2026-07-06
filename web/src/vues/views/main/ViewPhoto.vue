<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { computed, onBeforeUnmount, onMounted, onUnmounted, ref, watch } from 'vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'
import MediaViewer from '@/vues/components/viewer/MediaViewer.vue'
import { TimelineItem } from '@/scripts/types/generated/timeline.ts'
import { useTimelineStore } from '@/scripts/stores/timeline/timelineStore.ts'
import { useEventListener } from '@vueuse/core'
import MediaInfoPanel from '@/vues/components/viewer/components/MediaInfoPanel.vue'
import { makeDateTimeString, makeLocationString } from '@/scripts/utils.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useTheme } from 'vuetify/framework'

const props = withDefaults(
  defineProps<{
    disableEventCapture?: boolean | undefined
    overrideId?: string
    muted?: boolean
  }>(),
  {
    overrideId: undefined,
    muted: false,
    disableEventCapture: false,
  },
)

const route = useRoute()
const router = useRouter()
const theme = useTheme()
const mediaItemStore = useMediaItemStore()
const timelineStore = useTimelineStore()
const settings = useSettingStore()
const selectionStore = useSelectionStore()
const viewPhotoStore = useViewPhotoStore()
const dialogs = useDialogStore()
const authStore = useAuthStore()

const showRightButton = ref(false)
const showLeftButton = ref(false)
const persistentInfo = ref(false)
const hideSeconds = ref(7)
const infoMenuOpen = ref(false)
const optionsOpen = ref(false)
const isZoomed = ref(false)
const isPanoActive = ref(false)

const showUI = computed(() => hideSeconds.value > 0)
const hideTimer = setInterval(() => {
  hideSeconds.value--
  if (infoMenuOpen.value || optionsOpen.value) {
    hideSeconds.value = 10
  }
}, 1000)

useEventListener(document, 'mousemove', () => {
  hideSeconds.value = 10
})
useEventListener(document, 'click', () => {
  hideSeconds.value = 10
})

const id = computed(() => {
  if (props.overrideId) return props.overrideId
  const rawId = route.params.mediaId
  if (rawId && !Array.isArray(rawId)) return rawId
  console.warn('WEIRD ID IN ROUTE DETECTED')
  return null
})
const albumId = computed(() => route.params.albumId as string | undefined)

const isSelected = computed(() =>
  id.value === null ? false : selectionStore.selection.has(id.value),
)

const parentLocation = computed(() => {
  const parentRoute = route.matched[route.matched.length - 2]

  if (parentRoute && parentRoute.name) {
    const params = { ...route.params }
    delete params['mediaId']
    return {
      name: parentRoute.name,
      params,
      query: route.query,
    }
  } else {
    return { name: 'timeline' }
  }
})

const orderedIds = computed(() => {
  if (viewPhotoStore.ids.length > 0) return viewPhotoStore.ids
  return id.value ? [id.value] : []
})

const currentIndex = computed(() => orderedIds.value.findIndex((arrId) => id.value === arrId))
const nextId = computed(() => orderedIds.value[currentIndex.value + 1] ?? null)
const prevId = computed(() => orderedIds.value[currentIndex.value - 1] ?? null)
const isBin = computed(() => route.name === 'view-photo-bin')

const timestampString = computed(() => {
  const dateStr = fullImage.value?.taken_at_local
  if (!dateStr) return ''
  return makeDateTimeString(new Date(dateStr))
})

const locationString = computed(() => {
  if (!fullImage.value?.gps?.location) return ''
  const location = fullImage.value.gps.location
  const result = makeLocationString(location)
  return result ?? ''
})

const fullImage = computed(() => {
  if (!id.value) return undefined
  if (authStore.isAuthenticated) return mediaItemStore.mediaItems.get(id.value)
  else return mediaItemStore.sharedMediaItems.get(id.value)
})

const albumsForCurrentItem = computed(() => {
  if (!id.value || !authStore.isAuthenticated) return undefined
  return mediaItemStore.getAlbumsForMediaItem(id.value)
})

const timelineItem = computed<TimelineItem | undefined>(() => {
  if (!id.value) return undefined
  return timelineStore.mediaItemsMap.get(id.value)
})

const isVideo = computed<boolean>(
  () => fullImage.value?.is_video ?? timelineItem.value?.isVideo ?? false,
)

async function initialize() {
  const loadingId = id.value
  if (loadingId === null) return router.push(parentLocation.value)
  if (authStore.isAuthenticated) {
    await mediaItemStore.fetchMediaItem(loadingId)
  } else if (albumId.value) {
    await mediaItemStore.fetchSharedMediaItem(albumId.value, loadingId)
  }
  if (id.value !== loadingId) return
  console.log('FULL MEDIA ITEM', fullImage.value)
}

function toggleSelected() {
  if (!id.value) return
  selectionStore.toggleSelection(id.value)
}

function handleKeyDown(e: KeyboardEvent) {
  console.log(dialogs.customVisible, dialogs.anyVisible, dialogs.visible)
  if (dialogs.anyVisible) return
  if (e.key === 'ArrowLeft' && prevId.value) {
    e.preventDefault()
    router.replace({ path: `${viewPhotoStore.viewLink}${prevId.value}`, query: route.query })
  } else if (e.key === 'ArrowRight' && nextId.value) {
    e.preventDefault()
    router.replace({ path: `${viewPhotoStore.viewLink}${nextId.value}`, query: route.query })
  } else if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    router.push(parentLocation.value)
  }
}

function prefetchMediaItem(mediaItemId: string) {
  if (authStore.isAuthenticated) {
    mediaItemStore.fetchMediaItem(mediaItemId)
  } else if (albumId.value) {
    mediaItemStore.fetchSharedMediaItem(albumId.value, mediaItemId)
  }
}

onBeforeUnmount(() => clearInterval(hideTimer))
onMounted(() => document.addEventListener('keydown', handleKeyDown))
onUnmounted(() => document.removeEventListener('keydown', handleKeyDown))

// Pre-fetch
watch(prevId, () => {
  if (prevId.value) requestIdleCallback(() => prefetchMediaItem(prevId.value))
})
watch(nextId, () => {
  if (nextId.value) requestIdleCallback(() => prefetchMediaItem(nextId.value))
})

// Reset zoom states when content identifiers or viewer states change
watch(
  id,
  () => {
    isZoomed.value = false
    isPanoActive.value = false
    initialize()
  },
  { immediate: true },
)

watch(isVideo, () => {
  isZoomed.value = false
  isPanoActive.value = false
})
</script>

<template>
  <v-theme-provider
    :theme="settings.darkPhotoViewer ? 'dark' : theme.current.value.dark ? 'dark' : 'light'"
    with-background
    class="view-container"
    :class="{ 'backdrop-blur': settings.useBackdropBlur, 'hide-ui': !showUI }"
    :style="{
      backgroundColor: settings.useImageGlow ? 'rgb(var(--v-theme-background))' : 'black',
    }"
  >
    <media-viewer
      :disable-event-capture="disableEventCapture ?? false"
      :muted="muted ?? false"
      v-if="id"
      :is-video="isVideo"
      :media-item-id="id"
      :show-ui="showUI"
      class="photo-viewer"
      @zoom-change="isZoomed = $event"
      @pano-active="isPanoActive = $event"
    />
    <div class="top-bar">
      <div class="left-buttons">
        <v-btn
          :to="parentLocation"
          rounded="xl"
          icon="mdi-close"
          variant="plain"
          v-tooltip="{ text: 'Close viewer', location: 'bottom', attach: true, width: 140 }"
        />
        <v-btn
          rounded="xl"
          icon="mdi-view-gallery-outline"
          variant="plain"
          v-tooltip="{ text: 'Toggle gallery', location: 'bottom', attach: true, width: 140 }"
        />
      </div>
      <div class="top-main-text">
        <h3 v-if="fullImage?.user_caption">{{ fullImage.user_caption }}</h3>
        <router-link
          class="top-link"
          title="Go to date"
          :to="`/?highlight=${id}`"
          v-else-if="route.name !== 'view-photo-timeline'"
        >
          <h3>
            {{ timestampString }}
          </h3>
        </router-link>
        <h3 v-else>{{ timestampString }}</h3>
        <p>
          <router-link
            class="top-link location-link"
            v-if="fullImage?.gps"
            :to="`/map?lat=${fullImage.gps.latitude}&lon=${fullImage.gps.longitude}`"
            v-tooltip="{
              text: fullImage.gps.location?.country_name,
              disabled: fullImage.gps.location?.country_name === undefined,
              location: 'bottom',
              attach: true,
              width: 140,
            }"
          >
            <span>{{ locationString }}</span>
          </router-link>
          <template v-if="fullImage?.user_caption">
            <span class="ml-2 mr-2"> • </span>
            <span>{{ timestampString }}</span>
          </template>
          <span v-if="locationString.length > 0" class="ml-2 mr-2"> • </span>
          {{ (currentIndex + 1).toLocaleString() }} of
          {{ viewPhotoStore.ids.length === 0 ? '...' : viewPhotoStore.ids.length.toLocaleString() }}
        </p>
      </div>
      <div class="right-buttons">
        <v-btn
          v-if="settings.playMotionPhotos && fullImage?.media_features?.is_motion_photo"
          rounded="xl"
          icon="mdi-motion-play-outline"
          variant="plain"
          @click="viewPhotoStore.triggerPlayMotion"
          v-tooltip="{
            text: 'View photo in motion',
            location: 'bottom',
            attach: true,
            width: 200,
          }"
        />
        <v-btn
          v-if="selectionStore.selection.size > 0"
          :color="isSelected ? 'secondary' : 'default'"
          rounded="xl"
          :icon="isSelected ? 'mdi-check-circle' : 'mdi-checkbox-blank-circle-outline'"
          variant="plain"
          @click="toggleSelected"
          v-tooltip="{
            text: isSelected ? 'Remove from selection' : 'Add to selection',
            location: 'bottom',
            attach: true,
            width: 140,
          }"
        />

        <v-menu
          v-if="!isBin"
          :close-on-content-click="false"
          :persistent="persistentInfo"
          v-model="infoMenuOpen"
          location="bottom center"
        >
          <template v-slot:activator="{ props }">
            <v-btn
              :loading="fullImage === undefined"
              v-bind="props"
              rounded="xl"
              icon="mdi-information-outline"
              variant="plain"
              v-tooltip="{
                text: 'Extra info',
                location: 'bottom',
                attach: true,
                width: 140,
                disabled: infoMenuOpen,
              }"
            />
          </template>
          <media-info-panel
            :media-item="fullImage"
            :albums="albumsForCurrentItem"
            @open-date-time="persistentInfo = true"
            @close-date-time="persistentInfo = false"
          />
        </v-menu>
        <template v-if="authStore.isAuthenticated">
          <v-btn
            rounded="xl"
            icon="mdi-share-variant-outline"
            variant="plain"
            v-tooltip="{ text: 'Share', location: 'bottom', attach: true, width: 140 }"
          />
          <v-btn
            rounded="xl"
            icon="mdi-heart-outline"
            variant="plain"
            v-tooltip="{ text: 'Favourite', location: 'bottom', attach: true, width: 140 }"
          />
          <v-btn
            rounded="xl"
            icon="mdi-cloud-download-outline"
            variant="plain"
            v-tooltip="{ text: 'Download', location: 'bottom', attach: true, width: 140 }"
          />
          <v-btn
            rounded="xl"
            icon="mdi-trash-can-outline"
            variant="plain"
            v-tooltip="{ text: 'Move to bin', location: 'bottom', attach: true, width: 140 }"
          />
          <v-menu v-model="optionsOpen">
            <template v-slot:activator="{ props }">
              <v-btn rounded="xl" icon="mdi-dots-horizontal" variant="plain" v-bind="props" />
            </template>
            <v-list>
              <v-list-item>Hiiii</v-list-item>
            </v-list>
          </v-menu>
        </template>
      </div>
    </div>
    <div
      v-if="prevId !== null"
      class="prev-area"
      :class="{ 'zoomed-nav': isZoomed || isPanoActive }"
      @click="router.replace({ path: `${viewPhotoStore.viewLink}${prevId}`, query: route.query })"
      @mouseenter="showLeftButton = true"
      @mouseleave="showLeftButton = false"
    >
      <v-btn
        class="nav-btn"
        :class="{ 'show-btn': showLeftButton }"
        icon="mdi-chevron-left"
        variant="elevated"
        rounded="xl"
        size="70"
      ></v-btn>
    </div>
    <div
      v-if="nextId !== null"
      class="next-area"
      :class="{ 'zoomed-nav': isZoomed || isPanoActive }"
      @click="router.replace({ path: `${viewPhotoStore.viewLink}${nextId}`, query: route.query })"
      @mouseenter="showRightButton = true"
      @mouseleave="showRightButton = false"
    >
      <v-btn
        class="nav-btn"
        :class="{ 'show-btn': showRightButton }"
        icon="mdi-chevron-right"
        variant="elevated"
        rounded="xl"
        size="70"
      ></v-btn>
    </div>
  </v-theme-provider>
</template>

<style scoped>
.view-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 1400;
  --bg: var(--v-theme-surface-container-lowest);
  --fg: var(--v-theme-on-surface-container-lowest);
}

.hide-ui {
  cursor: none !important;
}

.hide-ui,
.hide-ui :deep(*) {
  cursor: none !important;
}

.photo-viewer {
  width: 100%;
  height: 100%;
  position: absolute;
  top: 0;
  left: 0;
  z-index: 1500;
  background-color: rgb(var(--bg));
}

.top-bar {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  display: flex;
  z-index: 1501;
  justify-content: space-between;
}

.left-buttons {
  display: flex;
  align-items: center;
  gap: 1px;
  padding: 0 30px;
  border-radius: 30px;
  margin: 10px;
  background-color: rgba(var(--bg), 0.8);
  box-shadow: 0 3px 12px rgba(var(--bg), 0.15);
  transition:
    background-color 0.15s,
    transform 0.5s;
}

.top-main-text {
  display: flex;
  text-align: center;
  justify-content: center;
  align-items: center;
  flex-direction: column;
  font-family: Jost, sans-serif;
  line-height: 0.9;
  padding: 10px 50px;
  margin: 10px;
  border-radius: 30px;
  background-color: rgba(var(--bg), 0.8);
  box-shadow: 0 3px 12px rgba(var(--bg), 0.15);
  transition:
    background-color 0.15s,
    transform 0.2s;
}

.top-main-text h3 {
  font-weight: 500;
  margin: 0;
  margin-bottom: 5px;
  font-size: 16px;
  color: rgba(var(--fg), 0.95);
}

.top-main-text p {
  margin: 0;
  font-weight: 300;
  color: rgba(var(--fg), 0.6);
  font-size: 13px;
}

.right-buttons {
  display: flex;
  align-items: center;
  gap: 1px;
  padding: 0 30px;
  margin: 10px;
  border-radius: 30px;
  background-color: rgba(var(--bg), 0.8);
  box-shadow: 0 3px 12px rgba(var(--bg), 0.15);
  transition:
    background-color 0.15s,
    transform 0.5s;
}

.backdrop-blur .right-buttons,
.backdrop-blur .left-buttons,
.backdrop-blur .top-main-text {
  backdrop-filter: blur(30px) saturate(150%) brightness(90%) contrast(90%);
  background-color: rgba(var(--bg), 0.5);
  border: 1px solid rgba(var(--fg), 0.1);
}

.backdrop-blur .right-buttons:hover,
.backdrop-blur .left-buttons:hover,
.backdrop-blur .top-main-text:hover {
  background-color: rgba(var(--bg), 0.7);
}

.hide-ui .right-buttons,
.hide-ui .left-buttons,
.hide-ui .top-main-text {
  transform: translateY(-80px);
}

.top-link {
  text-decoration: none;
  color: rgba(var(--fg), 0.8);
}

.top-link:active {
  color: inherit;
}

.top-link:hover {
  text-decoration: underline;
}

.location-link {
  color: rgba(var(--fg), 0.6);
}

.next-area {
  position: absolute;
  right: 0;
  top: 70px;
  height: calc(100% - 190px);
  width: 20%;
  min-width: 92px;
  max-width: 150px;
  cursor: pointer;
  display: flex;
  justify-content: flex-end;
  align-items: center;
  padding: 20px;
  z-index: 1502;
}

.prev-area {
  position: absolute;
  left: 0;
  top: 70px;
  height: calc(100% - 190px);
  width: 20%;
  min-width: 92px;
  max-width: 150px;
  cursor: pointer;
  display: flex;
  justify-content: flex-start;
  align-items: center;
  padding: 20px;
  z-index: 1502;
}

/* Custom transitions and styling for navigation buttons */
.nav-btn {
  opacity: 0;
  transition: opacity 0.15s ease-in-out;
  pointer-events: auto; /* Always keep the button itself interactive */
}

.nav-btn.show-btn {
  opacity: 1;
}

/* When zoomed in, disable pointer events on the large navigation container */
.prev-area.zoomed-nav,
.next-area.zoomed-nav {
  pointer-events: none;
  cursor: default;
}

/* Let the button itself show on hover in zoomed mode */
.prev-area.zoomed-nav .nav-btn:hover,
.next-area.zoomed-nav .nav-btn:hover {
  opacity: 1;
}
</style>
