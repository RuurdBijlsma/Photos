<script setup lang="ts">
import MdiClose from '~icons/mdi/close'
import MdiDelete from '~icons/mdi/delete'
import MdiDotsHorizontal from '~icons/mdi/dots-horizontal'
import MdiEarth from '~icons/mdi/earth'
import MdiLock from '~icons/mdi/lock'
import MdiPencilOutline from '~icons/mdi/pencil-outline'
import MdiPlus from '~icons/mdi/plus'
import MdiSortCalendarAscending from '~icons/mdi/sort-calendar-ascending'
import MdiSortCalendarDescending from '~icons/mdi/sort-calendar-descending'
import MdiSortClockAscendingOutline from '~icons/mdi/sort-clock-ascending-outline'
import MdiSortClockDescendingOutline from '~icons/mdi/sort-clock-descending-outline'
import { computed, nextTick, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDebounceFn, useTextareaAutosize } from '@vueuse/core'
import type { AlbumInfo } from '@/scripts/types/generated/timeline.ts'
import type { AlbumSort } from '@/scripts/types/api/album.ts'
import GlowThumbnail from '@/vues/components/ui/GlowThumbnail.vue'
import EditableTitle from '@/vues/components/ui/EditableTitle.vue'
import AlbumCollaborators from '@/vues/components/library/album/AlbumCollaborators.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { CURRENT_YEAR, MONTHS } from '@/scripts/constants.ts'

const props = defineProps<{
  albumId: string
  album: AlbumInfo | null
  initialName?: string | null
  initialDescription?: string | null
  thumbnailId: string | null
  isOwner: boolean
  isManualOrderMode: boolean
}>()

const emit = defineEmits<{
  startReorder: []
  deleteAlbum: []
}>()

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()
const albumStore = useAlbumStore()
const dialogs = useDialogStore()
const snackbars = useSnackbarsStore()

const albumTitle = ref<string | null>(props.initialName ?? props.album?.name ?? null)
const albumDescription = ref<string | null>(
  props.initialDescription ?? props.album?.description ?? '',
)
const descTextAreaFocus = ref(false)

const textAreaDescription = computed(() => albumDescription.value ?? '')
const { textarea, triggerResize } = useTextareaAutosize({ input: textAreaDescription })

const updateAlbumTitleDb = useDebounceFn(updateAlbumTitle, 500)
const updateAlbumDescriptionDb = useDebounceFn(updateAlbumDescription, 500)

function updateAlbumTitle(name: string) {
  if (!props.album || props.album.name === name) return

  if (route.query?.create === '1') {
    const newQuery = { ...route.query }
    delete newQuery.create
    router.replace({ query: newQuery })
  }
  props.album.name = name
  albumStore.updateAlbumDetails(props.albumId, { name })
}

function updateAlbumDescription(description: string) {
  if (props.album && props.album.description !== description) {
    props.album.description = description
    triggerResize()
    albumStore.updateAlbumDetails(props.albumId, { description })
  }
}

function setDescription() {
  albumDescription.value = ''
  nextTick(() => {
    textarea.value?.focus()
  })
}

function removeDescription() {
  albumStore.updateAlbumDetails(props.albumId, { description: null })
  albumDescription.value = null
}

async function publicize() {
  const confirmed = await dialogs.confirm({
    title: 'Make album public?',
    description:
      'Anyone with the link will be able to view this album, including all photos and videos it contains.',
    confirmText: 'Make public',
    icon: MdiEarth,
  })
  if (!confirmed || !props.album) return
  await albumStore.updateAlbumDetails(props.album.id, { isPublic: true })
  snackbars.success('Album is now available to anyone via link')
}

async function privatize() {
  const confirmed = await dialogs.confirm({
    title: 'Make album private?',
    description: 'It will no longer be publicly accessible via the URL.',
    confirmText: 'Make private',
    icon: MdiLock,
  })
  if (!confirmed || !props.album) return
  await albumStore.updateAlbumDetails(props.album.id, { isPublic: false })
  snackbars.success('Album is private')
}

const sortModeIcon = computed(() => {
  if (!props.album) return null
  const sortMode = props.album.sortMode as AlbumSort
  if (sortMode === 'AddedDesc') return MdiSortClockDescendingOutline
  if (sortMode === 'AddedAsc') return MdiSortClockAscendingOutline
  if (sortMode === 'DateAsc') return MdiSortCalendarAscending
  if (sortMode === 'DateDesc') return MdiSortCalendarDescending
  return null
})

const sortModeText = computed(() => {
  if (!props.album) return null
  const sortMode = props.album.sortMode as AlbumSort
  if (sortMode === 'AddedDesc') return 'Date added (new to old)'
  if (sortMode === 'AddedAsc') return 'Date added (old to new)'
  if (sortMode === 'DateAsc') return 'Capture date (old to new)'
  if (sortMode === 'DateDesc') return 'Capture date (new to old)'
  return null
})

const formattedDates = computed(() => {
  const firstDate = props.album?.firstDate ?? null
  const lastDate = props.album?.lastDate ?? null
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

  if (startYear !== endYear) {
    return `${getShortMonth(startMonthIdx)} ${startDay}, ${startYear} – ${getShortMonth(endMonthIdx)} ${endDay}, ${endYear}`
  }
  const yearSuffix = startYear === CURRENT_YEAR ? '' : `, ${startYear}`
  if (startMonthIdx !== endMonthIdx) {
    return `${getShortMonth(startMonthIdx)} ${startDay} – ${getShortMonth(endMonthIdx)} ${endDay}${yearSuffix}`
  }
  if (startDay === endDay) return `${MONTHS[startMonthIdx]} ${startDay}${yearSuffix}`
  return `${getShortMonth(startMonthIdx)} ${startDay} – ${endDay}${yearSuffix}`
})

watch(
  () => props.album?.name,
  (name) => {
    if (name) albumTitle.value = name
  },
)

watch(
  () => props.album?.description,
  (desc) => {
    if (desc !== undefined) albumDescription.value = desc
  },
)

watch(albumTitle, (newVal, oldVal) => {
  if (newVal && newVal !== oldVal) updateAlbumTitleDb(newVal)
})

watch(albumDescription, (newVal, oldVal) => {
  if (newVal !== oldVal && newVal !== null) {
    triggerResize()
    updateAlbumDescriptionDb(newVal ?? '')
  }
})
</script>

<template>
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
          <template v-slot:activator="{ props: menuProps }">
            <v-btn
              v-bind="menuProps"
              class="album-options-btn"
              :icon="MdiDotsHorizontal"
              variant="tonal"
              density="comfortable"
              color="primary"
              @click.stop.prevent
            />
          </template>
          <v-list density="compact" bg-color="surface-container-high">
            <v-list-item @click="emit('deleteAlbum')" :prepend-icon="MdiDelete">
              <v-list-item-title>Delete album</v-list-item-title>
            </v-list-item>
            <v-list-item @click="emit('startReorder')" :prepend-icon="MdiPencilOutline">
              <v-list-item-title>Edit order</v-list-item-title>
            </v-list-item>
          </v-list>
        </v-menu>
      </div>

      <p class="album-dates">
        <template v-if="sortModeIcon">
          <v-btn
            density="compact"
            v-if="authStore.isAuthenticated"
            icon
            variant="text"
            @click="emit('startReorder')"
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
            <v-icon size="20" :icon="album?.isPublic ? MdiEarth : MdiLock" />
          </v-btn>
          <span>•</span>
        </template>

        <span v-if="formattedDates">
          {{ formattedDates }}
        </span>
      </p>

      <div class="description-area" v-if="album">
        <v-btn
          v-if="albumDescription === null && isOwner"
          class="description-add-button"
          :prepend-icon="MdiPlus"
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
            @focus="descTextAreaFocus = true"
            @blur="descTextAreaFocus = false"
            v-if="isOwner"
          ></textarea>
          <p v-else>{{ albumDescription }}</p>
          <v-btn
            v-if="isOwner"
            @click="removeDescription"
            v-tooltip:top="'Remove description'"
            :icon="MdiClose"
            density="compact"
            variant="plain"
            :style="{
              opacity: descTextAreaFocus ? 'inherit' : 0,
            }"
          />
        </div>
      </div>

      <album-collaborators
        v-if="album"
        :album="album"
        :is-owner="isOwner"
        @publicize="publicize"
        @privatize="privatize"
      />
    </div>
  </div>
</template>

<style scoped>
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
</style>
