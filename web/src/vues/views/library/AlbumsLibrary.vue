<script setup lang="ts">
import { computed, onMounted } from 'vue'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import type { Album, AlbumSortField, SortDirection } from '@/scripts/types/api/album'
import { useRouter } from 'vue-router'
import { MONTHS } from '@/scripts/constants.ts'
import GlowThumbnail from '@/vues/components/ui/GlowThumbnail.vue'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useStorage } from '@vueuse/core'
import { useRefreshFunction } from '@/scripts/composables/useRefreshFunction.ts'
import { useDelayedBoolean } from '@/scripts/composables/useDelayedBoolean.ts'

const authStore = useAuthStore()
const dialogs = useDialogStore()
const router = useRouter()
const albumStore = useAlbumStore()

// Sorting State
const currentSortField = useStorage<AlbumSortField>(
  'albumLibrarySortField',
  'latestMediaItemTimestamp',
)
const currentSortDirection = useStorage<SortDirection>('albumLibrarySortDirection', 'desc')

const sortedAlbums = computed(() => {
  const albums = [...albumStore.userAlbums]
  const field = currentSortField.value
  const isAsc = currentSortDirection.value === 'asc'

  return albums.sort((a, b) => {
    const valA = a[field]
    const valB = b[field]
    // Keep albums with missing values at the end regardless of sort direction
    if (valA == null && valB == null) return 0
    if (valA == null) return 1
    if (valB == null) return -1

    let comparison: number
    if (field === 'name') {
      // case-insensitive string comparison
      comparison = String(valA).localeCompare(String(valB), undefined, {
        numeric: true,
        sensitivity: 'base',
      })
    } else {
      // Date fields
      const timeA = new Date(valA as string).getTime()
      const timeB = new Date(valB as string).getTime()
      comparison = timeA - timeB
    }
    return isAsc ? comparison : -comparison
  })
})

const flickerLoad = useDelayedBoolean(() => albumStore.userAlbumsLoading, 150)
const showLoading = computed(() => flickerLoad.value && sortedAlbums.value.length === 0)

// Separated Field Options
const sortFields = [
  { title: 'Name', field: 'name' },
  { title: 'Content date', field: 'latestMediaItemTimestamp' },
  { title: 'Updated date', field: 'updatedAt' },
]

const currentSortFieldTitle = computed(() => {
  return sortFields.find((f) => f.field === currentSortField.value)?.title || 'Sort'
})

// Dynamically select the correct icon depending on the field and direction
const sortDirectionIcon = computed(() => {
  if (currentSortField.value === 'name') {
    return currentSortDirection.value === 'asc'
      ? 'mdi-sort-alphabetical-ascending-variant'
      : 'mdi-sort-alphabetical-descending-variant'
  }
  if (currentSortField.value === 'updatedAt') {
    return currentSortDirection.value === 'asc'
      ? 'mdi-sort-clock-ascending-outline'
      : 'mdi-sort-clock-descending-outline'
  }
  return currentSortDirection.value === 'asc'
    ? 'mdi-sort-calendar-ascending'
    : 'mdi-sort-calendar-descending'
})

const sortDirectionTooltip = computed(() => {
  if (currentSortField.value === 'name') {
    return currentSortDirection.value === 'asc' ? 'A-Z' : 'Z-A'
  }
  return currentSortDirection.value === 'asc' ? 'Old to new' : 'New to old'
})

function handleFieldChange(field: AlbumSortField) {
  if (currentSortField.value !== field) {
    currentSortField.value = field
  }
}

function toggleDirection() {
  currentSortDirection.value = currentSortDirection.value === 'asc' ? 'desc' : 'asc'
}

async function makeNewAlbum() {
  await dialogs.alert({
    title: 'Create album',
    description: 'Create an album by selecting some photos and clicking "Add to album"',
    icon: 'mdi-image-album',
    actions: [
      {
        name: 'Go to photos',
        action: () => {
          router.push({ path: '/' })
        },
      },
    ],
  })
}

function getAlbumTimeSpan(album: Album) {
  if (!album.earliestMediaItemTimestamp || !album.latestMediaItemTimestamp) return ''
  const date1 = new Date(album.earliestMediaItemTimestamp)
  const date2 = new Date(album.latestMediaItemTimestamp)
  const year1 = date1.getFullYear()
  const year2 = date2.getFullYear()
  if (year1 === year2) {
    const month1 = MONTHS[date1.getMonth()]?.substring(0, 3)
    const month2 = MONTHS[date2.getMonth()]?.substring(0, 3)
    if (!month1 || !month2) return year1
    if (month1 === month2) {
      return `${month1} ${year1}`
    }
    return `${month1} - ${month2} ${year1}`
  }
  return `${year1} - ${year2}`
}

async function renameAlbum(album: Album) {
  await albumStore.renameAlbum(album.id, album.name)
  requestIdleCallback(() => albumStore.fetchUserAlbums())
}

async function deleteAlbum(albumId: string) {
  await albumStore.deleteAlbum(albumId)
  requestIdleCallback(() => albumStore.fetchUserAlbums())
}

async function leaveAlbum(albumId: string) {
  await albumStore.fetchAlbumMedia(albumId)
  const albumInfo = albumStore.albumMedia.get(albumId)
  if (!albumInfo) return
  const collaborators = albumInfo.album?.collaborators
  if (!collaborators) return
  const currentUserCollaborator = collaborators.find((c) => c.userId === authStore.user?.id)
  if (!currentUserCollaborator) return
  await albumStore.removeCollaborator(albumId, currentUserCollaborator.id, true)
  requestIdleCallback(() => albumStore.fetchUserAlbums())
}

onMounted(() => {
  albumStore.fetchUserAlbums()
})
useRefreshFunction(() => albumStore.fetchUserAlbums())
</script>

<template>
  <main-layout-container>
    <div class="library-container">
      <header class="library-header">
        <div class="header-left">
          <h1>Albums</h1>
          <span class="album-count">{{ sortedAlbums.length }} albums</span>
        </div>

        <div class="header-actions d-flex align-center">
          <v-menu location="bottom end">
            <template v-slot:activator="{ props }">
              <v-btn
                variant="text"
                color="primary"
                v-bind="props"
                rounded="xl"
                append-icon="mdi-chevron-down"
                class="text-none sort-text"
              >
                {{ currentSortFieldTitle }}
              </v-btn>
            </template>
            <v-list color="primary" density="compact">
              <v-list-item
                v-for="(option, index) in sortFields"
                :key="index"
                :title="option.title"
                :active="currentSortField === option.field"
                @click="handleFieldChange(option.field as AlbumSortField)"
              />
            </v-list>
          </v-menu>

          <!-- Direction Toggle (Right) -->
          <v-btn
            variant="text"
            color="primary"
            class="sort-direction-button"
            :icon="sortDirectionIcon"
            @click="toggleDirection"
            v-tooltip="{
              location: 'top',
              text: sortDirectionTooltip,
            }"
          />

          <v-btn
            color="primary"
            prepend-icon="mdi-plus"
            rounded
            variant="flat"
            class="text-none ml-3 new-album"
            @click="makeNewAlbum"
          >
            New Album
          </v-btn>
        </div>
      </header>

      <!-- Loading State -->
      <div v-if="showLoading" class="loading-state">
        <v-progress-circular indeterminate color="primary" size="48" />
      </div>

      <!-- Empty State -->
      <div
        v-else-if="!albumStore.userAlbumsLoading && sortedAlbums.length === 0"
        class="empty-state"
      >
        <v-icon icon="mdi-image-album" size="100" class="mb-4 opacity-20" />
        <h2>No albums yet</h2>
        <p>Create your first album to start organizing your memories.</p>
        <v-btn color="primary" variant="tonal" rounded class="mt-6" @click="makeNewAlbum">
          Create Album
        </v-btn>
      </div>

      <!-- Grid Layout -->
      <div v-else class="album-grid">
        <router-link
          v-for="album in sortedAlbums"
          :key="album.id"
          :to="`/album/${album.id}`"
          class="album-card"
          @mouseenter="albumStore.fetchAlbumMedia(album.id)"
        >
          <div class="album-image">
            <glow-thumbnail
              class="album-glow-image"
              :media-item-id="album.thumbnailId"
              :height="200"
              :width="200"
              border-radius="20px"
              :strength="0.7"
            />
            <v-menu location="bottom end" v-if="album.ownerId === authStore.user?.id">
              <template v-slot:activator="{ props }">
                <v-btn
                  v-bind="props"
                  class="album-options-btn"
                  icon="mdi-dots-horizontal"
                  variant="flat"
                  density="comfortable"
                  color="primary"
                  @click.stop.prevent
                />
              </template>
              <v-list density="compact">
                <v-list-item @click="renameAlbum(album)">
                  <v-list-item-title>Rename album</v-list-item-title>
                </v-list-item>
                <v-list-item @click="deleteAlbum(album.id)">
                  <v-list-item-title>Delete album</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
            <v-menu v-else location="bottom end">
              <template v-slot:activator="{ props }">
                <v-avatar
                  v-tooltip="{
                    location: 'top',
                    text: 'Shared album',
                  }"
                  size="35"
                  v-bind="props"
                  @click.stop.prevent
                  class="album-shared-avatar"
                  color="primary"
                >
                  <v-icon icon="mdi-share" size="23" />
                </v-avatar>
              </template>
              <v-list density="compact">
                <v-list-item @click="leaveAlbum(album.id)">
                  <v-list-item-title>Leave shared album</v-list-item-title>
                </v-list-item>
              </v-list>
            </v-menu>
          </div>

          <div class="album-info">
            <h3
              v-tooltip="{
                location: 'top',
                text: album.name || 'Untitled Album',
                disabled: album.name.length <= 19,
              }"
              class="album-name text-truncate"
            >
              {{ album.name || 'Untitled Album' }}
            </h3>
            <p class="album-meta">
              <span>
                {{ album.mediaCount.toLocaleString() ?? 0 }} item{{
                  album.mediaCount === 1 ? '' : 's'
                }}
              </span>
              •
              <span>{{ getAlbumTimeSpan(album) }}</span>
            </p>
          </div>
        </router-link>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.library-container {
  padding: 20px 20px;
}

.library-header {
  padding: 0 10px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 30px;
}

.header-left h1 {
  margin: 0;
  font-size: 2.5rem;
  font-weight: 600;
  line-height: 1.2;
}

.album-count {
  font-size: 0.9rem;
  font-weight: 400;
  color: rgb(var(--v-theme-on-surface-variant));
}

.loading-state {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 100px 0;
}

.album-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 30px;
  justify-items: center;
}

.album-image {
  position: relative;
  height: 200px;
  width: 200px;
}

.album-shared-avatar {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 5;
}

.album-options-btn {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 5;
  opacity: 0;
}

.album-image:hover .album-options-btn {
  opacity: 1;
}

.album-glow-image {
  top: 0;
  left: 0;
  position: absolute;
  width: 100%;
  height: 100%;
}

.album-card {
  text-decoration: none;
  color: inherit;
  transition: transform 0.2s ease;
}

.album-card:hover {
  transform: translateY(-5px) scale(1.01);
}

.album-info {
  margin-top: 12px;
  padding: 0 4px;
}

.album-name {
  font-size: 1rem;
  font-weight: 600;
  margin-bottom: 2px;
  max-width: 195px;
  margin-top: 0;
}

.album-meta {
  font-size: 0.85rem;
  opacity: 0.6;
  margin-top: 0;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 100px 0;
  text-align: center;
}

.empty-state h2 {
  opacity: 0.8;
}

.empty-state p {
  opacity: 0.6;
}

.album-card:hover :deep(.glow-image-container) {
  box-shadow: 0 10px 30px -10px rgba(var(--v-theme-primary), 0.3);
}

.new-album {
  font-weight: 600 !important;
}
</style>
