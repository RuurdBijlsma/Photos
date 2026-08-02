import { defineStore } from 'pinia'
import type { SystemStats } from '@/scripts/types/api/system.ts'
import systemService from '@/scripts/services/systemService.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useObjStorage } from '@/scripts/utils.ts'

const DEFAULT_SYSTEM_STATS: SystemStats = {
  hasClusteredPeople: true,
  hasClusteredPhotos: true,
  allowFileDeletion: true,
  allowFileModifications: true,
  isIngesting: false,
  disk: {
    areSameDrive: true,
    mediaDrive: {
      diskAvailable: 50,
      diskTotal: 100,
      diskUsed: 0,
    },
    appDataDrive: {
      diskAvailable: 50,
      diskTotal: 100,
      diskUsed: 0,
    },
  },
}

export const useSystemStore = defineStore('system', () => {
  const snackbarStore = useSnackbarsStore()

  const stats = useObjStorage<SystemStats>('systemStats', DEFAULT_SYSTEM_STATS)

  async function fetchStats() {
    try {
      const { data } = await systemService.getStats()
      stats.value = data
    } catch (e) {
      snackbarStore.error('Could not fetch system stats', e)
    }
  }

  return {
    stats,
    fetchStats,
  }
})
