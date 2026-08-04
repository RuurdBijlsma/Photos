import { defineStore } from 'pinia'
import { computed, shallowRef } from 'vue'
import type {
  FullMediaItem,
  MediaItemAlbumRef,
  MediaItemWithAlbums,
} from '@/scripts/types/api/fullPhoto.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import type { AxiosResponse } from 'axios'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import type { UpdateMediaItemRequest } from '@/scripts/types/api/mediaItem.ts'
import albumService from '@/scripts/services/albumService.ts'
import type { SharedMediaItem } from '@/scripts/types/api/album.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

export const useMediaItemStore = defineStore('mediaItem', () => {
  const snackbarStore = useSnackbarsStore()
  const authStore = useAuthStore()

  const mediaItems = shallowRef(new Map<string, FullMediaItem>())
  const mediaItemAlbums = shallowRef(new Map<string, MediaItemAlbumRef[]>())
  const mediaItemPromises = new Map<string, Promise<AxiosResponse<MediaItemWithAlbums>>>()
  const sharedMediaItems = shallowRef(new Map<string, SharedMediaItem>())
  const sharedMediaItemPromises = new Map<string, Promise<AxiosResponse<SharedMediaItem>>>()
  const anyMediaItems = computed(() => {
    if (authStore.isAuthenticated) {
      return mediaItems.value
    } else {
      return sharedMediaItems.value
    }
  })

  async function updateMediaItem(mediaItemId: string, itemDetails: UpdateMediaItemRequest) {
    try {
      await mediaItemService.update(mediaItemId, itemDetails)
      requestIdleCallback(() => fetchMediaItem(mediaItemId, false))
    } catch (e) {
      snackbarStore.error(`Failed to update album.`, e as Error)
    }
  }

  async function fetchMediaItem(id: string, useCache: boolean = true) {
    if (mediaItemPromises.has(id)) {
      await mediaItemPromises.get(id)
      return
    }
    if (useCache && mediaItems.value.has(id) && mediaItemAlbums.value.has(id)) return

    const promise = mediaItemService.getMediaItem(id)
    mediaItemPromises.set(id, promise)
    const result = await promise
    mediaItemPromises.delete(id)

    // Re-assign a new Map instance so Vue's computed properties detect the reference change
    const updatedMediaItems = new Map(mediaItems.value)
    updatedMediaItems.set(id, result.data.media_item)
    mediaItems.value = updatedMediaItems

    const updatedAlbums = new Map(mediaItemAlbums.value)
    updatedAlbums.set(id, result.data.albums)
    mediaItemAlbums.value = updatedAlbums
  }

  async function fetchSharedMediaItem(
    albumId: string,
    mediaItemId: string,
    useCache: boolean = true,
  ) {
    if (sharedMediaItemPromises.has(mediaItemId)) {
      await sharedMediaItemPromises.get(mediaItemId)
      return
    }
    if (useCache && sharedMediaItems.value.has(mediaItemId)) return

    const promise = albumService.getSharedMediaItem(albumId, mediaItemId)
    sharedMediaItemPromises.set(mediaItemId, promise)
    const result = await promise
    sharedMediaItemPromises.delete(mediaItemId)

    // Re-assign a new Map instance
    const updatedShared = new Map(sharedMediaItems.value)
    updatedShared.set(mediaItemId, result.data)
    sharedMediaItems.value = updatedShared
  }

  function getAlbumsForMediaItem(id: string): MediaItemAlbumRef[] | undefined {
    return mediaItemAlbums.value.get(id)
  }

  return {
    mediaItems,
    mediaItemAlbums,
    sharedMediaItems,

    fetchSharedMediaItem,
    fetchMediaItem,
    updateMediaItem,
    getAlbumsForMediaItem,
    anyMediaItems,
  }
})
