<script setup lang="ts">
import MdiPencilOutline from '~icons/mdi/pencil-outline'
import { computed, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue'
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from 'vue-router'
import type { AlbumSort } from '@/scripts/types/api/album.ts'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import albumService from '@/scripts/services/albumService.ts'
import { useRefreshFunction } from '@/scripts/composables/useRefreshFunction.ts'
import { usePageTitle } from '@/scripts/composables/usePageTitle.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import AlbumHeader from '@/vues/components/library/album/AlbumHeader.vue'
import AlbumReorderToolbar from '@/vues/components/library/album/AlbumReorderToolbar.vue'
import AlbumEmptyState from '@/vues/components/library/album/AlbumEmptyState.vue'

const route = useRoute()
const router = useRouter()
const simpleTimeline = useTemplateRef('simpleTimeline')
const albumStore = useAlbumStore()
const authStore = useAuthStore()
const snackbars = useSnackbarsStore()
const dialogs = useDialogStore()

const isInitialLoad = ref(true)
const isManualOrderMode = ref(false)
const localItems = ref<SimpleTimelineItem[]>([])
const pendingSortMode = ref<AlbumSort>('None')

const id = computed(() => {
  const rawId = route.params.albumId
  if (rawId && !Array.isArray(rawId)) return rawId
  console.warn('WEIRD ALBUM ID IN ROUTE DETECTED')
  return null
})

const albumResponse = computed(() =>
  id.value ? (albumStore.albumMedia.get(id.value) ?? null) : null,
)
const minimalAlbumInfo = computed(() => albumStore.userAlbums.find((a) => a.id === id.value))
const album = computed(() => albumResponse.value?.album ?? null)
const items = computed(() => albumResponse.value?.items ?? [])
const displayedItems = computed(() => (isManualOrderMode.value ? localItems.value : items.value))

const isOwner = computed(() => authStore.user?.id === album.value?.ownerId)
const thumbnailId = computed(
  () => album.value?.thumbnailId ?? minimalAlbumInfo.value?.thumbnailId ?? null,
)
const albumPageTitle = computed(() => album.value?.name ?? minimalAlbumInfo.value?.name ?? 'Album')

// --- Reordering Logic ---

function manualOrderMode() {
  isManualOrderMode.value = true
  localItems.value = [...items.value]
  pendingSortMode.value = (album.value?.sortMode ?? 'None') as AlbumSort
  snackbars.enqueue({
    message: 'Drag photos to reorder the album',
    icon: MdiPencilOutline,
    timeout: 5000,
  })
}

function cancelReorder() {
  isManualOrderMode.value = false
  localItems.value = []
  pendingSortMode.value = 'None'
}

async function saveReorder() {
  if (!id.value) return
  await albumStore.reorderMedia(
    id.value,
    localItems.value.map((i) => i.id),
    pendingSortMode.value,
  )
  isManualOrderMode.value = false
}

function onReorder(newOrder: SimpleTimelineItem[]) {
  localItems.value = newOrder
  pendingSortMode.value = 'None'
}

async function fetchSortedPreview(mode: AlbumSort) {
  if (!id.value) return
  if (!isManualOrderMode.value) manualOrderMode()

  const response = await albumService.getSortedMedia(id.value, mode)
  localItems.value = response.items
  pendingSortMode.value = mode
  simpleTimeline.value?.setOrder(localItems.value)
}

async function deleteAlbum() {
  if (!album.value?.id) return
  const deleted = await albumStore.deleteAlbum(album.value.id)
  if (deleted) await router.push({ name: 'albums' })
}

async function confirmRouteLeave() {
  if (!isManualOrderMode.value) return true

  const confirmed = await dialogs.confirm({
    title: 'Leave reorder mode?',
    description: 'You have unsaved changes. Are you sure you want to leave?',
    confirmText: 'Leave',
    color: 'warning',
  })
  if (confirmed) {
    isManualOrderMode.value = false
    return true
  }
  return false
}

// --- Import Polling Logic ---

let importPollInterval: number | null = null

function clearImportPoll() {
  if (importPollInterval) {
    clearInterval(importPollInterval)
    importPollInterval = null
  }
}

function startImportPoll() {
  clearImportPoll()
  importPollInterval = setInterval(() => {
    if (id.value) {
      albumStore.fetchAlbumMedia(id.value, false)
      albumStore.fetchUserAlbums()
    }
  }, 5000)
}

onBeforeRouteLeave(confirmRouteLeave)
onBeforeRouteUpdate(confirmRouteLeave)

onBeforeUnmount(() => {
  clearImportPoll()
})

watch(albumResponse, () => {
  if (albumResponse.value !== null) {
    isInitialLoad.value = false
  }
})

watch(
  () => route.query.importing,
  (importing) => {
    if (importing === 'true') {
      startImportPoll()
    } else {
      clearImportPoll()
    }
  },
  { immediate: true },
)

let lastItemsCount = 0
watch(items, () => {
  const importing = route.query.importing === 'true'
  if (items.value.length !== 0 && items.value.length === lastItemsCount && importing) {
    router.replace({ query: {} })
  }
  lastItemsCount = items.value.length
})

watch(
  id,
  () => {
    isManualOrderMode.value = false
    simpleTimeline.value?.scrollToTop()
    if (!id.value) return
    albumStore.fetchAlbumMedia(id.value, false)
  },
  { immediate: true },
)

usePageTitle(albumPageTitle, { fallback: 'Album' })
useRefreshFunction(() => {
  if (id.value) albumStore.fetchAlbumMedia(id.value, false)
})
</script>

<template>
  <div class="album-container">
    <album-reorder-toolbar
      v-if="isManualOrderMode"
      :current-sort-mode="pendingSortMode"
      @cancel="cancelReorder"
      @save="saveReorder"
      @sort-preview="fetchSortedPreview"
    />

    <simple-timeline
      ref="simpleTimeline"
      :timeline-items="displayedItems"
      :view-link="`/album/${id}/view/`"
      v-if="id"
      :context="{ album: album ?? undefined }"
      :is-manual-order-mode="isManualOrderMode"
      @reorder="onReorder"
    >
      <album-header
        :album-id="id"
        :album="album"
        :initial-name="minimalAlbumInfo?.name"
        :initial-description="minimalAlbumInfo?.description"
        :thumbnail-id="thumbnailId"
        :is-owner="isOwner"
        :is-manual-order-mode="isManualOrderMode"
        @start-reorder="manualOrderMode"
        @delete-album="deleteAlbum"
      />

      <album-empty-state
        v-if="displayedItems.length === 0 && !isInitialLoad"
        :is-importing="route.query.importing === 'true'"
      />
    </simple-timeline>
  </div>
</template>

<style scoped>
.album-container {
  display: flex;
  flex-direction: column;
  width: 100%;
}
</style>
