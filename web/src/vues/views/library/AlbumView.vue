<script setup lang="ts">
import { useRoute, useRouter, onBeforeRouteLeave, onBeforeRouteUpdate } from 'vue-router'
import { computed, nextTick, ref, useTemplateRef, watch } from 'vue'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import type { AlbumSort } from '@/scripts/types/api/album.ts'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useDebounceFn, useTextareaAutosize } from '@vueuse/core'
import { onBeforeUnmount } from 'vue'
import EditableTitle from '@/vues/components/ui/EditableTitle.vue'
import { CURRENT_YEAR, MONTHS } from '@/scripts/constants.ts'
import albumService from '@/scripts/services/albumService.ts'
// eslint-disable-next-line
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import GlowThumbnail from '@/vues/components/ui/GlowThumbnail.vue'
import UserAvatar from '@/vues/components/ui/UserAvatar.vue'
import userService from '@/scripts/services/userService.ts'
import type { SmallUser } from '@/scripts/types/api/user.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { copyToClipboard } from '@/scripts/utils.ts'

const route = useRoute()
const router = useRouter()
const simpleTimeline = useTemplateRef('simpleTimeline')
const albumStore = useAlbumStore()
const snackbars = useSnackbarsStore()
const dialogs = useDialogStore()
const authStore = useAuthStore()

const isInitialLoad = ref(true)
const isManualOrderMode = ref(false)
const collabMenuOpen = ref(false)
const localItems = ref<SimpleTimelineItem[]>([])
const pendingSortMode = ref<AlbumSort>('None')
const allUsers = ref<SmallUser[]>([])
const otherUsers = computed(() =>
  allUsers.value.filter((u) => !album.value?.collaborators.map((c) => c.userId).includes(u.id)),
)
const currentUserCollaborator = computed(() => {
  if (!album.value) return undefined
  return album.value.collaborators.find((c) => c.userId === authStore.user?.id)
})

const isOwner = computed(() => authStore.user?.id === album.value?.ownerId)
const id = computed(() => {
  const rawId = route.params.albumId
  if (rawId && !Array.isArray(rawId)) return rawId
  console.warn('WEIRD ALBUM ID IN ROUTE DETECTED')
  return null
})
const albumResponse = computed(() => {
  if (!id.value) return null
  const response = albumStore.albumMedia.get(id.value) ?? null
  console.log('AlbumResponse', response)
  return response
})
const minimalAlbumInfo = computed(() => albumStore.userAlbums.find((a) => a.id === id.value))
const album = computed(() => albumResponse.value?.album ?? null)
const items = computed(() => albumResponse.value?.items ?? [])
const displayedItems = computed(() => (isManualOrderMode.value ? localItems.value : items.value))
const updateAlbumTitleDb = useDebounceFn(updateAlbumTitle, 500)
const updateAlbumDescriptionDb = useDebounceFn(updateAlbumDescription, 500)
const albumTitle = ref<string | null>(null)
const albumDescription = ref<string | null>('')
const thumbnailId = computed(() => {
  if (album.value !== null) return album.value?.thumbnailId ?? null
  return minimalAlbumInfo.value?.thumbnailId ?? null
})
const sortModeIcon = computed(() => {
  if (!album.value) return null
  const sortMode = album.value.sortMode as AlbumSort
  if (sortMode === 'AddedDesc') {
    return 'mdi-sort-clock-descending-outline'
  } else if (sortMode === 'AddedAsc') {
    return 'mdi-sort-clock-ascending-outline'
  } else if (sortMode === 'DateAsc') {
    return 'mdi-sort-calendar-ascending'
  } else if (sortMode === 'DateDesc') {
    return 'mdi-sort-calendar-descending'
  }
  return null
})
const sortModeText = computed(() => {
  if (!album.value) return null
  const sortMode = album.value.sortMode as AlbumSort
  if (sortMode === 'AddedDesc') {
    return 'Date added (new to old)'
  } else if (sortMode === 'AddedAsc') {
    return 'Date added (old to new)'
  } else if (sortMode === 'DateAsc') {
    return 'Capture date (old to new)'
  } else if (sortMode === 'DateDesc') {
    return 'Capture date (new to old)'
  }
  return null
})
const formattedDates = computed(() => {
  const firstDate = album.value?.firstDate ?? null
  const lastDate = album.value?.lastDate ?? null
  if (firstDate === null || lastDate === null) return null
  const start = new Date(firstDate)
  const end = new Date(lastDate)

  const startYear = start.getFullYear()
  const endYear = end.getFullYear()
  const startMonthIdx = start.getMonth()
  const endMonthIdx = end.getMonth()
  const startDay = start.getDate()
  const endDay = end.getDate()
  const getShortMonth = (idx: number) => MONTHS[idx]?.slice(0, 3)
  if (startYear !== endYear)
    return `${getShortMonth(startMonthIdx)} ${startDay}, ${startYear} – ${getShortMonth(endMonthIdx)} ${endDay}, ${endYear}`
  const yearSuffix = startYear === CURRENT_YEAR ? '' : `, ${startYear}`
  if (startMonthIdx !== endMonthIdx)
    return `${getShortMonth(startMonthIdx)} ${startDay} – ${getShortMonth(endMonthIdx)} ${endDay}${yearSuffix}`
  if (startDay === endDay) return `${MONTHS[startMonthIdx]} ${startDay}${yearSuffix}`
  return `${getShortMonth(startMonthIdx)} ${startDay} – ${endDay}${yearSuffix}`
})
const textAreaDescription = computed(() => albumDescription.value ?? '')
const { textarea, triggerResize } = useTextareaAutosize({ input: textAreaDescription })
const descTextAreaFocus = ref(false)

function updateAlbumTitle(name: string) {
  if (album.value === null || id.value === null) return
  if (album.value.name === name) return
  const create = route.query?.create
  if (create === '1') {
    const newQuery = { ...route.query }
    delete newQuery.create
    router.replace({ query: newQuery })
  }
  album.value.name = name
  albumStore.updateAlbumDetails(id.value, { name })
}

function updateAlbumDescription(albumId: string, description: string) {
  if (id.value && albumId === id.value && album.value && album.value.description !== description) {
    console.log(album.value.description, description)
    album.value.description = description
    triggerResize()
    albumStore.updateAlbumDetails(id.value, { description })
  }
}

function setDescription() {
  albumDescription.value = ''
  nextTick(() => {
    textarea.value?.focus()
  })
}

function focusTextArea() {
  descTextAreaFocus.value = true
}

function blurTextArea() {
  descTextAreaFocus.value = false
}

function removeDescription() {
  if (!id.value) return
  albumStore.updateAlbumDetails(id.value, { description: null })
  albumDescription.value = null
}

async function deleteAlbum() {
  const id = album.value?.id
  if (!id) return
  const deleted = await albumStore.deleteAlbum(id)
  if (deleted) await router.push({ name: 'albums' })
}

function manualOrderMode() {
  isManualOrderMode.value = true
  localItems.value = [...items.value]
  pendingSortMode.value = (album.value?.sortMode ?? 'None') as AlbumSort
  snackbars.enqueue({
    message: 'Drag photos to reorder the album',
    icon: 'mdi-pencil-outline',
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

function onReorder(items: SimpleTimelineItem[]) {
  localItems.value = items
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

async function confirmRouteLeave() {
  if (isManualOrderMode.value) {
    const confirmed = await dialogs.confirm({
      title: 'Leave reorder mode?',
      description: 'You have unsaved changes. Are you sure you want to leave?',
      confirmText: 'Leave',
      color: 'warning',
    })
    if (confirmed) {
      isManualOrderMode.value = false
      return true
    } else {
      return false
    }
  } else {
    return true
  }
}

async function fetchUserList() {
  try {
    const { data } = await userService.listUsers()
    allUsers.value = data
  } catch (e) {
    snackbars.error("Can't fetch other users", e)
  }
}

async function addCollaborator(userId: number) {
  if (!album.value) return

  try {
    await albumService.addCollaborator(album.value.id, {
      userId,
      role: 'Contributor',
    })
    requestIdleCallback(() => {
      if (album.value) albumStore.fetchAlbumMedia(album.value.id, false)
    })
  } catch (e) {
    snackbars.error('Could not add collaborator', e)
  }
}

async function crossServerInvite() {
  if (!album.value) return
  try {
    const { data: token } = await albumService.generateInvite(album.value.id)
    await dialogs.alert({
      title: 'Cross-Server Share',
      description: `
        <div>
          <p>Send this token to someone. They can paste it into the search bar on their own Ruurd Photos instance to import this album.</p>
          <br>
          <code class="share-link">${token}</code>
        </div>
      `,
      actions: [
        {
          action: () => copyToClipboard(token),
          name: 'Copy Token',
        },
        { action: () => ({}), name: 'Done' },
      ],
    })
  } catch (e) {
    snackbars.error("Can't generate invite", e)
    return
  }
}

async function publicize() {
  const confirmed = await dialogs.confirm({
    title: 'Make album public?',
    description:
      'Anyone with the link will be able to view this album, including all photos and videos it contains.',
    confirmText: 'Make public',
    icon: 'mdi-earth',
  })
  if (!confirmed || !album.value) return
  await albumStore.updateAlbumDetails(album.value.id, { isPublic: true })
  snackbars.success('Album is now available to anyone via link')
}

async function privatize() {
  const confirmed = await dialogs.confirm({
    title: 'Make album private?',
    description: 'It will no longer be publicly accessible via the URL.',
    confirmText: 'Make private',
    icon: 'mdi-lock',
  })
  if (!confirmed || !album.value) return
  await albumStore.updateAlbumDetails(album.value.id, { isPublic: false })
  snackbars.success('Album is private')
}

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
  // When items count hasn't changed for 5 seconds, while not being 0 -> stop refreshing
  if (items.value.length !== 0 && items.value.length === lastItemsCount && importing) {
    router.replace({ query: {} })
  }
  lastItemsCount = items.value.length
})

watch(
  id,
  () => {
    isManualOrderMode.value = false
    albumTitle.value = null
    albumDescription.value = null
    console.log('Album ID change', id.value)
    simpleTimeline.value?.scrollToTop()
    if (!id.value) return
    albumStore.fetchAlbumMedia(id.value, false)
  },
  { immediate: true },
)

watch(
  minimalAlbumInfo,
  () => {
    // Do on nextTick, otherwise albumTitle may still be set from previous album page
    // it's unset in the `id` watcher
    nextTick(() => {
      if (!id.value) return
      if (!minimalAlbumInfo.value) return
      if (albumTitle.value === null) albumTitle.value = minimalAlbumInfo.value.name
      if (albumDescription.value === null)
        albumDescription.value = minimalAlbumInfo.value.description
    })
  },
  { immediate: true },
)

watch(albumDescription, (newVal, oldVal) => {
  if (newVal !== oldVal && id.value && newVal !== null) {
    triggerResize()
    updateAlbumDescriptionDb(id.value, newVal ?? '')
  }
})

watch(albumTitle, (newVal, oldVal) => {
  if (newVal && newVal !== oldVal) updateAlbumTitleDb(newVal)
})

watch(
  () => album.value?.name,
  () => {
    const name = album.value?.name ?? null
    if (name === null) return
    albumTitle.value = name
  },
)

watch(collabMenuOpen, () => {
  if (!collabMenuOpen.value) return
  fetchUserList()
})
</script>

<template>
  <div class="album-container">
    <div class="album-reorder-header" v-if="isManualOrderMode">
      <div class="reorder-header-title">
        <v-icon icon="mdi-pencil" class="mr-5" size="25" />
        <p>Edit album order</p>
      </div>
      <div>
        <v-menu location="bottom start">
          <template v-slot:activator="{ props }">
            <v-btn
              v-bind="props"
              class="mr-5"
              variant="text"
              rounded
              color="primary"
              prepend-icon="mdi-sort"
            >
              Sort photos
            </v-btn>
          </template>
          <v-list density="compact" bg-color="surface-container-high">
            <v-list-item
              v-if="album"
              :active="album.sortMode == 'DateDesc'"
              @click="fetchSortedPreview('DateDesc')"
              prepend-icon="mdi-sort-calendar-descending"
            >
              <v-list-item-title>Newest first</v-list-item-title>
            </v-list-item>
            <v-list-item
              v-if="album"
              :active="album.sortMode == 'DateAsc'"
              @click="fetchSortedPreview('DateAsc')"
              prepend-icon="mdi-sort-calendar-ascending"
            >
              <v-list-item-title>Oldest first</v-list-item-title>
            </v-list-item>
            <v-divider />
            <v-list-item
              @click="fetchSortedPreview('AddedDesc')"
              prepend-icon="mdi-sort-clock-descending-outline"
            >
              <v-list-item-title>Recently added</v-list-item-title>
            </v-list-item>
            <v-list-item
              @click="fetchSortedPreview('AddedAsc')"
              prepend-icon="mdi-sort-clock-ascending-outline"
            >
              <v-list-item-title>Oldest added</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-menu>
        <v-btn variant="text" rounded class="mr-2" @click="cancelReorder">Cancel</v-btn>
        <v-btn
          variant="tonal"
          color="primary"
          rounded
          @click="saveReorder"
          prepend-icon="mdi-check"
        >
          Save
        </v-btn>
      </div>
    </div>
    <simple-timeline
      ref="simpleTimeline"
      :timeline-items="displayedItems"
      :view-link="`/album/${id}/view/`"
      v-if="id"
      :context="{ album: album ?? undefined }"
      :is-manual-order-mode="isManualOrderMode"
      @reorder="onReorder"
    >
      <div class="album-header">
        <div class="album-header-left">
          <glow-thumbnail
            v-if="thumbnailId"
            :media-item-id="thumbnailId"
            border-radius="44px"
            :height="222"
            :max-width="(222 * 16) / 9"
          />
        </div>
        <div class="album-header-right">
          <div class="album-header-first-row">
            <editable-title
              class="editable-header"
              v-if="albumTitle !== null && isOwner"
              name="album title"
              :autofocus="route.query?.create === '1'"
              v-model="albumTitle"
            />
            <h1
              class="non-owner-title"
              v-else-if="albumTitle !== null && !isOwner && authStore.isAuthenticated"
            >
              {{ albumTitle }}
            </h1>
            <h1 class="non-owner-title" v-else-if="!authStore.isAuthenticated">
              {{ album?.name }}
            </h1>
            <v-menu location="bottom end" v-if="!isManualOrderMode && isOwner">
              <template v-slot:activator="{ props }">
                <v-btn
                  v-bind="props"
                  class="album-options-btn"
                  icon="mdi-dots-horizontal"
                  variant="tonal"
                  density="comfortable"
                  color="primary"
                  @click.stop.prevent
                />
              </template>
              <v-list density="compact" bg-color="surface-container-high">
                <v-list-item @click="deleteAlbum" prepend-icon="mdi-delete">
                  <v-list-item-title>Delete album</v-list-item-title>
                </v-list-item>
                <v-list-item @click="manualOrderMode" prepend-icon="mdi-pencil-outline">
                  <v-list-item-title>Edit order</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
          </div>
          <p v-if="formattedDates" class="album-dates">
            <template v-if="sortModeIcon">
              <v-btn
                density="compact"
                v-if="authStore.isAuthenticated"
                icon
                variant="text"
                @click="manualOrderMode()"
                v-tooltip="{
                  location: 'top',
                  text: sortModeText,
                }"
              >
                <v-icon size="20" :icon="sortModeIcon" />
              </v-btn>
              <v-icon
                v-else
                size="20"
                class="mt-1 mb-1"
                :icon="sortModeIcon"
                v-tooltip="{
                  location: 'top',
                  text: sortModeText,
                }"
              />
              <span>•</span>
            </template>
            <template v-if="authStore.isAuthenticated">
              <v-btn
                density="compact"
                icon
                variant="text"
                @click="album?.isPublic ? privatize() : publicize()"
                v-tooltip="{
                  location: 'top',
                  text: album?.isPublic ? 'Publicly available' : 'Private',
                }"
              >
                <v-icon size="20" :icon="album?.isPublic ? `mdi-earth` : `mdi-lock`" />
              </v-btn>
              <span>•</span>
            </template>
            <span>
              {{ formattedDates }}
            </span>
          </p>
          <div class="description-area" v-if="album">
            <v-btn
              v-if="albumDescription === null && isOwner"
              class="description-add-button"
              prepend-icon="mdi-plus"
              density="compact"
              variant="plain"
              rounded
              @click="setDescription"
            >
              Add description
            </v-btn>
            <div v-else-if="albumDescription !== null" class="description-with-remove">
              <textarea
                class="album-description"
                ref="textarea"
                :style="{
                  boxShadow:
                    albumDescription?.length > 0
                      ? ''
                      : `inset 0 0 0 1px rgba(var(--v-theme-on-background), 0.3)`,
                }"
                v-model="albumDescription"
                @input="triggerResize"
                @focus="focusTextArea"
                @blur="blurTextArea"
                v-if="isOwner"
              ></textarea>
              <p v-else>{{ albumDescription }}</p>
              <v-btn
                v-if="isOwner"
                @click="removeDescription"
                v-tooltip:top="'Remove description'"
                icon="mdi-close"
                density="compact"
                variant="plain"
                :style="{
                  opacity: descTextAreaFocus ? 'inherit' : 0,
                }"
              ></v-btn>
            </div>
          </div>
          <div class="album-collaborators" v-if="album">
            <v-menu v-for="collaborator in album.collaborators" :key="collaborator.id">
              <template v-slot:activator="{ props }">
                <user-avatar
                  v-bind="collaborator.userId !== album.ownerId && isOwner ? props : undefined"
                  :name="collaborator.name"
                  :avatar-id="collaborator.avatarId"
                  class="collaborator-avatar"
                  v-tooltip:top="
                    `${collaborator.name}${collaborator.userId === album.ownerId ? ' (Owner)' : ''}`
                  "
                />
              </template>
              <v-list density="compact">
                <v-list-item
                  @click="albumStore.removeCollaborator(album.id, collaborator.id, false)"
                >
                  <v-list-item-title>Remove {{ collaborator.name }}</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
            <v-menu v-model="collabMenuOpen" v-if="isOwner">
              <template v-slot:activator="{ props }">
                <v-btn
                  v-bind="props"
                  v-tooltip:top="'Add collaborator'"
                  icon="mdi-share"
                  variant="tonal"
                  color="primary"
                  size="40"
                  :loading="allUsers.length === 0 && collabMenuOpen"
                ></v-btn>
              </template>
              <v-list density="compact">
                <template v-if="otherUsers.length > 0">
                  <v-list-subheader class="mb-1">Add a collaborator</v-list-subheader>
                  <v-list-item
                    @click="addCollaborator(user.id)"
                    v-for="user in otherUsers"
                    :key="user.id"
                    class="collab-list-item"
                  >
                    <user-avatar :name="user.name" :avatar-id="user.avatarId" />
                    <v-list-item-title>{{ user.name }}</v-list-item-title>
                  </v-list-item>
                  <v-divider class="mt-2 mb-1" />
                </template>
                <v-list-item
                  prepend-icon="mdi-lock"
                  @click="privatize"
                  v-if="album.isPublic"
                  v-tooltip="{
                    location: 'top',
                    text: 'Only invited people will be able to access this album.',
                  }"
                >
                  <v-list-item-title>Make private</v-list-item-title>
                </v-list-item>
                <v-list-item
                  prepend-icon="mdi-earth"
                  @click="publicize"
                  v-else
                  v-tooltip="{
                    location: 'top',
                    text: 'Anyone with the link will be able to view this album.',
                  }"
                >
                  <v-list-item-title>Make public</v-list-item-title>
                </v-list-item>
                <v-list-item
                  prepend-icon="mdi-cloud-upload"
                  @click="crossServerInvite"
                  v-tooltip="{
                    location: 'top',
                    text: 'Export this album to another server.',
                  }"
                >
                  <v-list-item-title>Share with other server</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
            <v-menu v-else-if="currentUserCollaborator">
              <template v-slot:activator="{ props }">
                <v-btn
                  v-bind="props"
                  v-tooltip:top="'Leave album'"
                  icon="mdi-minus"
                  variant="tonal"
                  color="primary"
                  size="40"
                ></v-btn>
              </template>
              <v-list density="compact">
                <v-list-item
                  @click="albumStore.removeCollaborator(album.id, currentUserCollaborator.id, true)"
                  class="collab-list-item"
                >
                  <v-list-item-title>Leave album</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
          </div>
        </div>
      </div>
      <div class="empty-album" v-if="displayedItems.length === 0 && !isInitialLoad">
        <v-icon color="on-surface-variant" size="200" icon="mdi-image-album"></v-icon>
        <h2>This album is empty</h2>
        <template v-if="route.query.importing === 'true'">
          <p>Importing media from remote server...</p>
          <v-progress-circular class="mt-5" indeterminate size="50" />
        </template>
      </div>
    </simple-timeline>
  </div>
</template>

<style scoped>
.empty-album {
  height: 700px;
  width: 100%;
  display: flex;
  place-items: center;
  place-content: center;
  flex-direction: column;
  color: rgb(var(--v-theme-on-surface-variant));
}

.album-container {
  display: flex;
  flex-direction: column;
  width: 100%;
}

.album-reorder-header {
  display: flex;
  padding: 20px 25px;
  border-radius: 50px;
  width: calc(100% - 50px);
  margin-right: 50px;
  margin-top: 10px;
  margin-bottom: 10px;
  background-color: rgb(var(--v-theme-background));
  box-shadow: inset 0 0 0 1px rgba(var(--v-theme-primary), 0.1);
  justify-content: space-between;
  align-items: center;
}

.reorder-header-title {
  display: flex;
  align-items: center;
}

.reorder-header-title p {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.album-header {
  display: flex;
  width: 100%;
}

.album-header-left {
  padding: 10px;
}

.album-header-right {
  padding: 20px;
  flex-grow: 1;
}

.album-header-first-row {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.album-options-btn {
  margin: 4px;
}

.editable-header {
  margin-left: -19px;
  width: calc(100% + 19px);
}

.non-owner-title {
  margin: 0;
  font-weight: 500;
  font-size: 50px;
  line-height: 1.2;
  color: rgb(var(--v-theme-on-background));
  padding: 5px 3px;
}

.album-dates {
  gap: 5px;
  font-weight: 400;
  font-size: 15px;
  opacity: 0.7;
  margin: 0;
  display: flex;
  align-items: center;
}

.description-add-button {
  margin-left: -15px;
  font-style: italic;
  text-transform: capitalize;
  font-weight: 400;
  opacity: 0.5;
  margin-top: 4px;
  margin-bottom: 5px;
}

.description-with-remove {
  display: flex;
  align-items: center;
  gap: 10px;
}

.album-description {
  font-weight: 400;
  font-size: 14px;
  opacity: 0.8;
  margin-top: 3px;
  resize: none;
  border-radius: 10px;
  padding: 5px;
  margin-left: -5px;
  width: calc(100% + 5px);
  font-family: Roboto, sans-serif;
  margin-bottom: 3px;
  position: relative;
  background-color: transparent;
  border: 0 solid transparent;
}

.album-description:focus {
  outline: 1px solid rgb(var(--v-theme-primary));
}

.album-collaborators {
  margin-top: 5px;
  display: flex;
  gap: 10px;
}

.collaborator-avatar {
  font-weight: 600;
}

.collab-list-item:deep(.v-list-item__content) {
  display: flex;
  align-items: center;
  gap: 20px;
}
</style>
