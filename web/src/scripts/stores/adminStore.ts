import { ref, type Ref } from 'vue'
import { defineStore } from 'pinia'
import adminService from '@/scripts/services/adminService.ts'
import type {
  DiskResponse,
  MediaSampleResponse,
  UnsupportedFilesResponse,
  AdminUserInfo,
  JobInfo,
} from '@/scripts/types/api/admin.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

export const useAdminStore = defineStore('admin', () => {
  // --- STATE ---
  const disks: Ref<DiskResponse | null> = ref(null)
  const isLoading: Ref<boolean> = ref(false)
  const folders: Ref<string[] | null> = ref(null)
  const mediaSamples: Ref<MediaSampleResponse | null> = ref(null)
  const unsupportedFiles: Ref<UnsupportedFilesResponse | null> = ref(null)
  // Jobs
  const jobs: Ref<JobInfo[]> = ref([])
  const totalJobs: Ref<number> = ref(0)
  const isJobsLoading: Ref<boolean> = ref(false)

  // Administration state
  const users: Ref<AdminUserInfo[]> = ref([])
  const isUsersLoading: Ref<boolean> = ref(false)

  const snackbarStore = useSnackbarsStore()
  const authStore = useAuthStore()

  async function fetchDiskInfo() {
    isLoading.value = true
    try {
      const response = await adminService.getDisks()
      disks.value = response.data
    } catch (error) {
      snackbarStore.error('Failed to fetch disk info', error)
    } finally {
      isLoading.value = false
    }
  }

  async function fetchJobs(query: {
    page?: number
    limit?: number
    offset?: number
    sort?: string[]
    filter?: string[]
  }) {
    isJobsLoading.value = true
    try {
      const params = new URLSearchParams()
      if (query.limit !== undefined) params.append('limit', query.limit.toString())
      if (query.offset !== undefined) params.append('offset', query.offset.toString())
      if (query.page !== undefined) params.append('page', query.page.toString())

      if (query.sort) {
        query.sort.forEach((s) => params.append('sort', s))
      }
      if (query.filter) {
        query.filter.forEach((f) => params.append('filter', f))
      }

      const response = await adminService.getJobs(params)
      jobs.value = response.data.data
      totalJobs.value = response.data.total
    } catch (error) {
      snackbarStore.error('Failed to load background jobs list', error)
      throw error
    } finally {
      isJobsLoading.value = false
    }
  }

  async function cancelJob(jobId: number) {
    try {
      await adminService.cancelJob(jobId)
      snackbarStore.success(`Job #${jobId} cancelled successfully`)
    } catch (error) {
      snackbarStore.error(`Failed to cancel Job #${jobId}`, error)
      throw error
    }
  }

  async function retryJob(jobId: number) {
    try {
      await adminService.retryJob(jobId)
      snackbarStore.success(`Job #${jobId} scheduled for retry`)
    } catch (error) {
      snackbarStore.error(`Failed to retry Job #${jobId}`, error)
      throw error
    }
  }

  // --- USER ADMINISTRATION ACTIONS ---

  async function fetchUsers() {
    isUsersLoading.value = true
    try {
      const response = await adminService.getAdminUsers()
      users.value = response.data
    } catch (error) {
      snackbarStore.error('Failed to fetch user list', error)
    } finally {
      isUsersLoading.value = false
    }
  }

  async function updateUserFolder(userId: number, userFolder: string) {
    try {
      await adminService.updateUserMediaFolder(userId, userFolder)

      // Update locally
      const user = users.value.find((u) => u.id === userId)
      if (user) {
        user.mediaFolder = userFolder
      }

      // Update logged-in user if changing our own folder
      if (authStore.user && authStore.user.id === userId) {
        authStore.user.mediaFolder = userFolder
      }
    } catch (error) {
      snackbarStore.error('Failed to update media folder', error)
      throw error
    }
  }

  async function deleteUser(userId: number) {
    try {
      await adminService.deleteUser(userId)
      // Filter out locally
      users.value = users.value.filter((u) => u.id !== userId)
    } catch (error) {
      snackbarStore.error('Failed to delete user', error)
      throw error
    }
  }

  // --- RETURN ---
  return {
    // State
    disks,
    isLoading,
    folders,
    mediaSamples,
    unsupportedFiles,
    users,
    isUsersLoading,
    // Actions
    fetchDiskInfo,
    fetchUsers,
    updateUserMediaFolder: updateUserFolder,
    deleteUser,
    // New Jobs State & Action
    jobs,
    totalJobs,
    isJobsLoading,
    fetchJobs,
    cancelJob,
    retryJob,
  }
})
