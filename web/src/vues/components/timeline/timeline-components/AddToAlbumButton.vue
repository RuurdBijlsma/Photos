<script setup lang="ts">
import { computed, ref } from 'vue'
import albumService from '@/scripts/services/albumService.ts'
import { useRouter } from 'vue-router'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import type { Album } from '@/scripts/types/api/album.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import ItemsPreview from '@/vues/components/timeline/timeline-components/ItemsPreview.vue'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

const props = withDefaults(
  defineProps<{
    excludeAlbumIds?: string[]
    idsToAdd: string[]
  }>(),
  {
    excludeAlbumIds: () => [],
  },
)

const router = useRouter()
const snackbarStore = useSnackbarsStore()
const selectionStore = useSelectionStore()
const albumStore = useAlbumStore()

const show = ref(false)
const newLoading = ref(false)
const addLoading = ref(new Set<string>())
const filteredUserAlbums = computed(() =>
  albumStore.userAlbums.filter((a) => !props.excludeAlbumIds.includes(a.id)),
)

async function createNew() {
  if (props.idsToAdd.length === 0) {
    return snackbarStore.warning("Can't create album with no items in it.")
  }
  newLoading.value = true
  try {
    const { data: album } = await albumService.createAlbum({
      name: '',
      isPublic: false,
      mediaItemIds: props.idsToAdd,
    })
    requestIdleCallback(() => albumStore.fetchUserAlbums().then())
    await router.push({ path: '/album/' + album.id, query: { create: '1' } })
    selectionStore.selection = new Set()
  } catch (e) {
    snackbarStore.error('Error creating album', e as Error)
  } finally {
    newLoading.value = true
  }
}

async function addToAlbum(album: Album) {
  if (props.idsToAdd.length === 0) {
    return snackbarStore.warning("Can't add 0 items to album.")
  }
  addLoading.value.add(album.id)
  try {
    await albumService.addMediaToAlbum(album.id, { mediaItemIds: props.idsToAdd })
    requestIdleCallback(() => {
      albumStore.fetchAlbumMedia(album.id, false)
      albumStore.fetchUserAlbums()
    })
    selectionStore.selection = new Set()
    snackbarStore.info(
      `${props.idsToAdd.length} item${props.idsToAdd.length === 1 ? '' : 's'} added`,
      {
        label: album.name,
        onClick: () => {
          router.push({ path: '/album/' + album.id })
        },
      },
    )
  } catch (e) {
    snackbarStore.error('Error adding to album', e as Error)
  } finally {
    addLoading.value.delete(album.id)
  }
}
</script>

<template>
  <v-menu v-model="show" :close-on-content-click="false" location="top center" :offset="[25, 0]">
    <template v-slot:activator="{ props }">
      <v-btn
        v-bind="props"
        :icon="show ? 'mdi-chevron-up' : 'mdi-plus'"
        variant="plain"
        density="compact"
        :disabled="show"
        v-tooltip:top="'Add to album'"
      />
    </template>

    <v-card color="surface-container-low" min-width="300" flat class="album-picker rounded-xl pa-3">
      <v-card-title class="text-center mb-2 title-text">Add to album</v-card-title>
      <items-preview :media-item-ids="idsToAdd" />
      <v-list class="mt-3 albums-list" v-if="filteredUserAlbums.length > 0">
        <v-list-item rounded v-for="album in filteredUserAlbums" :key="album.id">
          <template v-slot:prepend>
            <v-avatar rounded color="surface-container-high">
              <thumbnail-img
                v-if="album.thumbnailId"
                :media-item-id="album.thumbnailId"
                :height="144"
              />
              <v-icon v-else icon="mdi-image-album" color="primary" class="opacity-70" />
            </v-avatar>
          </template>
          <div class="list-content">
            <div class="list-left">
              <v-list-item-title v-tooltip:top="album.name" v-if="album.name !== ''">{{
                album.name
              }}</v-list-item-title>
              <v-list-item-title v-else><i class="opacity-50">Unnamed</i></v-list-item-title>
              <v-list-item-subtitle>
                {{ album.mediaCount }} item{{ album.mediaCount === 1 ? '' : 's' }}
              </v-list-item-subtitle>
            </div>
            <v-list-item-action>
              <v-btn
                variant="plain"
                @click="addToAlbum(album)"
                :loading="addLoading.has(album.id)"
                class="rounded-pill"
                density="compact"
                icon="mdi-plus"
                v-tooltip:top="'Add'"
              />
            </v-list-item-action>
          </div>
        </v-list-item>
      </v-list>
      <v-card-actions class="card-actions">
        <v-btn @click="createNew" rounded class="px-5" :loading="newLoading">
          <v-icon icon="mdi-plus" class="mr-2"></v-icon> Create new album
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-menu>
</template>

<style scoped>
.album-picker {
  border-radius: 50px;
  overflow-x: hidden !important;
}

.title-text {
  color: rgb(var(--v-theme-primary));
}

.card-actions {
  display: flex;
  justify-content: center;
}

.albums-list {
  background-color: rgba(var(--v-theme-surface-container));
  width: calc(100% + 24px);
  margin-left: -12px;
  max-height: 200px;
}

.list-content {
  display: flex;
  justify-content: space-between;
}

.list-left {
  pointer-events: none;
  max-width: 170px;
}

.card-actions {
  padding: 12px 0 0;
  margin: 0 !important;
}
</style>
