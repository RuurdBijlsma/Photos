<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import type { Album, AlbumSortField, SortDirection } from '@/scripts/types/api/album'
import albumService from '@/scripts/services/albumService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useRouter } from 'vue-router'
import { MONTHS } from '@/scripts/constants.ts'
import GlowThumbnail from '@/vues/components/ui/GlowThumbnail.vue'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useStorage } from '@vueuse/core'

const snackbarStore = useSnackbarsStore()
const authStore = useAuthStore()
const dialogs = useDialogStore()
const router = useRouter()
const albumStore = useAlbumStore()

const loading = ref(false)
const showSkeleton = ref(false)
let skeletonTimeout: ReturnType<typeof setTimeout> | null = null

// Sorting State
const currentSortField = useStorage<AlbumSortField>('albumLibrarySortField', 'latestPhoto')
const currentSortDirection = useStorage<SortDirection>('albumLibrarySortDirection', 'desc')
const userAlbums = ref<Album[]>([])

// Separated Field Options
const sortFields = [
  { title: 'Name', field: 'name' },
  { title: 'Content date', field: 'latestPhoto' },
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

async function loadAlbums() {
  loading.value = true
  showSkeleton.value = false

  // Clear any existing timeout to avoid race conditions
  if (skeletonTimeout) clearTimeout(skeletonTimeout)

  // Set skeleton to appear only after 150ms
  skeletonTimeout = setTimeout(() => {
    showSkeleton.value = true
  }, 150)

  try {
    const { data } = await albumService.getUserAlbums(
      currentSortField.value,
      currentSortDirection.value,
    )
    console.log(data)
    userAlbums.value = data
  } catch (e) {
    snackbarStore.error('Could not fetch albums', e)
  } finally {
    loading.value = false
    showSkeleton.value = false
    if (skeletonTimeout) clearTimeout(skeletonTimeout)
  }
}

function handleFieldChange(field: AlbumSortField) {
  if (currentSortField.value !== field) {
    currentSortField.value = field
    loadAlbums()
  }
}

function toggleDirection() {
  currentSortDirection.value = currentSortDirection.value === 'asc' ? 'desc' : 'asc'
  loadAlbums()
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
  requestIdleCallback(() => loadAlbums())
}

async function deleteAlbum(albumId: string) {
  await albumStore.deleteAlbum(albumId)
  requestIdleCallback(() => loadAlbums())
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
  requestIdleCallback(() => loadAlbums())
}

onMounted(() => {
  loadAlbums()
})

onUnmounted(() => {
  // Prevent memory leaks / UI state issues if the component mounts/unmounts quickly
  if (skeletonTimeout) clearTimeout(skeletonTimeout)
})
</script>

<template>
  <main-layout-container>
    <div class="library-container">
      <header class="library-header">
        <div class="header-left">
          <h1>Albums</h1>
          <span class="album-count">{{ userAlbums.length }} albums</span>
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

      <!-- Grid Layout -->
      <div v-if="showSkeleton" class="album-grid">
        <div v-for="i in 9" :key="i" class="album-card-skeleton">
          <v-skeleton-loader
            type="card"
            elevation="1"
            color="surface-container-low"
            height="265"
            width="200"
            class="rounded-xl"
          />
        </div>
      </div>

      <div v-else class="album-grid">
        <router-link
          v-for="album in userAlbums"
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
                  ><v-icon icon="mdi-share" size="23"></v-icon
                ></v-avatar>
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
              <span
                >{{ album.mediaCount.toLocaleString() ?? 0 }} item{{
                  album.mediaCount === 1 ? '' : 's'
                }}</span
              >
              •
              <span>{{ getAlbumTimeSpan(album) }}</span>
            </p>
          </div>
        </router-link>
      </div>

      <!-- Empty State -->
      <div v-if="!loading && userAlbums.length === 0" class="empty-state">
        <v-icon icon="mdi-image-album" size="100" class="mb-4 opacity-20" />
        <h2>No albums yet</h2>
        <p>Create your first album to start organizing your memories.</p>
        <v-btn color="primary" variant="tonal" rounded class="mt-6" @click="makeNewAlbum">
          Create Album
        </v-btn>
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

/* Custom shadow/glow effect on hover similar to GlowImage's logic */
.album-card:hover :deep(.glow-image-container) {
  box-shadow: 0 10px 30px -10px rgba(var(--v-theme-primary), 0.3);
}

.new-album {
  font-weight: 600 !important;
}
</style>
