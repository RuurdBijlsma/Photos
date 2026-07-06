import { defineStore } from 'pinia'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import userService from '@/scripts/services/userService.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

export const useProfileStore = defineStore('profile', () => {
  const snackbarStore = useSnackbarsStore()
  const authStore = useAuthStore()

  async function setProfilePic(mediaItemId: string) {
    try {
      await userService.updateProfile({ avatarId: mediaItemId })
      requestIdleCallback(() => authStore.fetchCurrentUser())
    } catch (e) {
      snackbarStore.error("Couldn't set profile picture", e)
    }
  }

  return {
    setProfilePic,
  }
})
