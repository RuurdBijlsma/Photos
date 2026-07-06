import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { BackupInfo } from '@/scripts/types/api/album.ts'
import albumService from '@/scripts/services/albumService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'

export const useAlbumBackupStore = defineStore('albumBackup', () => {
  const backups = ref<BackupInfo[]>([])
  const snackbarStore = useSnackbarsStore()
  const dialogs = useDialogStore()
  const albumStore = useAlbumStore()

  async function fetchBackups() {
    try {
      const { data } = await albumService.listBackups()
      backups.value = data
    } catch (e) {
      snackbarStore.error("Can't fetch backups", e)
    }
  }

  async function restoreBackup(backupFilename: string) {
    const confirmed = await dialogs.confirm({
      title: 'Restore Album Backup?',
      description: `Are you sure you want to restore the backup "${backupFilename}"? This will recreate missing albums and merge media configurations.`,
      confirmText: 'Restore',
      color: 'warning',
    })
    if (!confirmed) return false

    try {
      await albumService.restoreBackup(backupFilename)
      snackbarStore.enqueue({ message: 'Backup restored successfully', icon: 'mdi-backup-restore' })
      requestIdleCallback(() => albumStore.fetchUserAlbums())
      return true
    } catch (e) {
      snackbarStore.error('Error restoring backup', e)
      return false
    }
  }

  return {
    fetchBackups,
    restoreBackup,
    backups,
  }
})
