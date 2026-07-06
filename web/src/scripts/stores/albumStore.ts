import { defineStore } from 'pinia'
import { shallowRef, triggerRef } from 'vue'
import type { Album, AlbumSort, UpdateAlbumRequest } from '@/scripts/types/api/album.ts'
import albumService from '@/scripts/services/albumService.ts'
import type { FullAlbumMediaResponse } from '@/scripts/types/generated/timeline.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useRouter } from 'vue-router'
import { useObjStorage } from '@/scripts/utils.ts'

export const useAlbumStore = defineStore('album', () => {
  const snackbarStore = useSnackbarsStore()
  const selectionStore = useSelectionStore()
  const dialogs = useDialogStore()
  const router = useRouter()

  const userAlbums = useObjStorage<Album[]>('userAlbums', [])
  const albumMedia = shallowRef(new Map<string, FullAlbumMediaResponse>())
  const albumMediaPromises = new Map<string, Promise<FullAlbumMediaResponse>>()

  async function fetchUserAlbums() {
    try {
      const { data } = await albumService.getUserAlbums()
      userAlbums.value = data
    } catch (e) {
      snackbarStore.error("Can't fetch user albums", e)
    }
  }

  async function fetchAlbumMedia(albumId: string, useCache = true) {
    if (albumMediaPromises.has(albumId)) {
      await albumMediaPromises.get(albumId)!
      return
    }
    if (useCache && albumMedia.value.has(albumId)) return

    const promise = albumService.getAlbumMedia(albumId)
    albumMediaPromises.set(albumId, promise)
    const response = await promise
    albumMediaPromises.delete(albumId)

    albumMedia.value.set(albumId, response)
    triggerRef(albumMedia)
  }

  async function deleteAlbum(albumId: string) {
    const confirmed = await dialogs.confirm({
      title: 'Are you sure?',
      description: 'This will permanently delete the album.',
      confirmText: 'Delete',
      color: 'error',
    })
    if (!confirmed) return false
    console.warn('DELETING', { confirmed, albumId })
    try {
      await albumService.deleteAlbum(albumId)
      snackbarStore.enqueue({ message: 'Album deleted', icon: 'mdi-delete' })
      requestIdleCallback(() => fetchUserAlbums())
    } catch (e) {
      snackbarStore.error('Error deleting album', e)
    }
    return true
  }

  async function renameAlbum(albumId: string, currentName: string) {
    const newName = await dialogs.prompt({
      title: 'Rename Album',
      defaultValue: currentName,
      confirmText: 'Rename',
    })
    if (currentName === newName || !newName) return
    try {
      await albumService.updateAlbum(albumId, { name: newName })
      requestIdleCallback(() => fetchUserAlbums())
    } catch (e) {
      snackbarStore.error('Error renaming album', e)
    }
  }

  async function updateAlbumDetails(albumId: string, albumDetails: UpdateAlbumRequest) {
    try {
      await albumService.updateAlbum(albumId, albumDetails)
      requestIdleCallback(() => {
        fetchAlbumMedia(albumId, false)
        fetchUserAlbums()
      })
    } catch (e) {
      snackbarStore.error(`Failed to update album.`, e as Error)
    }
  }

  async function removeFromAlbum(albumId: string, removeItemIds: string[]) {
    const confirmed = await dialogs.confirm({
      title: 'Are you sure?',
      description: `This will remove ${removeItemIds.length} item${removeItemIds.length === 1 ? '' : 's'} from the album.`,
      confirmText: 'Remove',
      color: 'error',
    })
    if (!confirmed) return
    try {
      await albumService.removeMediaFromAlbum(albumId, removeItemIds)
      selectionStore.deselectMany(removeItemIds)
      requestIdleCallback(() => {
        fetchAlbumMedia(albumId, false)
        fetchUserAlbums()
      })
    } catch (e) {
      snackbarStore.error('Error removing items from album', e)
    }
  }

  async function reorderMedia(albumId: string, mediaItemIds: string[], sortMode: AlbumSort) {
    try {
      await albumService.reorderMedia(albumId, mediaItemIds, sortMode)
      requestIdleCallback(() => fetchAlbumMedia(albumId, false))
    } catch (e) {
      snackbarStore.error('Error reordering items', e)
    }
  }

  async function removeCollaborator(
    albumId: string,
    collaboratorId: number,
    isSelfRemove: boolean = false,
  ) {
    try {
      await albumService.removeCollaborator(albumId, collaboratorId)
      if (isSelfRemove) {
        requestIdleCallback(() => fetchUserAlbums())
        snackbarStore.success('Successfully removed yourself from album')
        await router.push('/albums')
      } else {
        requestIdleCallback(() => fetchAlbumMedia(albumId, false))
      }
    } catch (e) {
      snackbarStore.error('Could not remove collaborator', e)
    }
  }

  return {
    userAlbums,
    albumMedia,

    fetchUserAlbums,
    fetchAlbumMedia,
    updateAlbumDetails,
    removeFromAlbum,
    reorderMedia,
    deleteAlbum,
    renameAlbum,
    removeCollaborator,
  }
})
