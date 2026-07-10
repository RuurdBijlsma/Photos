import { computed, ref, type Ref, watch } from 'vue'
import { defineStore } from 'pinia'
import authService from '@/scripts/services/authService.ts'
import type { CreateUser, LoginUser, User } from '@/scripts/types/api/auth.ts'
import { useRouter } from 'vue-router'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { useIntervalFn } from '@vueuse/core'
import { useObjStorage } from '@/scripts/utils.ts'

type AuthStatus = 'idle' | 'loading' | 'error' | 'success'

export const useAuthStore = defineStore('auth', () => {
  const systemStore = useSystemStore()

  // --- STATE ---
  const user = useObjStorage<User | null>('authUser', null)
  const status: Ref<AuthStatus> = ref('idle')
  const router = useRouter()

  // --- GETTERS ---
  const isAuthenticated = computed(() => !!user.value)
  const isAdmin = computed(() => user.value?.role === 'admin')

  // --- ACTIONS ---

  /**
   * Refreshes the access token using the HttpOnly refresh token cookie.
   * If it fails, it will trigger a logout and throw an error.
   */
  async function refreshTokens(): Promise<void> {
    try {
      await authService.refreshSession()
      requestIdleCallback(fetchCurrentUser)
    } catch (error) {
      console.warn('[refresh errored] call logout()', error)
      await logout()
      throw error
    }
  }

  const { pause: pauseFetchingStats, resume: resumeFetchingStats } = useIntervalFn(
    () => {
      if (isAuthenticated.value) {
        systemStore.fetchStats().then()
      }
    },
    15000,
    { immediate: false },
  )

  watch(
    () => isAuthenticated.value,
    (val) => {
      if (val) {
        resumeFetchingStats()
      } else {
        pauseFetchingStats()
      }
    },
  )

  async function onAuthenticated() {
    await systemStore.fetchStats()
    resumeFetchingStats()
  }

  /**
   * Fetches the current user's data using the access token cookie.
   */
  async function fetchCurrentUser() {
    const response = await authService.getMe()
    user.value = response.data
  }

  /**
   * Logs the user in, sets state, and gets user data.
   */
  async function login(credentials: LoginUser) {
    status.value = 'loading'
    try {
      await authService.login(credentials)
      await fetchCurrentUser()
      status.value = 'success'
    } catch (error) {
      if (error instanceof Error) console.warn('Failed to login. ' + error.message, error)
      status.value = 'error'
      throw error
    }
  }

  /**
   * Registers the user, and then logs in.
   */
  async function register(credentials: CreateUser): Promise<User> {
    status.value = 'loading'
    try {
      const response = await authService.register(credentials)
      // Log in automatically after successful registration
      await login({ email: credentials.email, password: credentials.password })
      status.value = 'success'
      return response.data
    } catch (error) {
      status.value = 'error'
      throw error
    }
  }

  /**
   * Logs the user out, clears all auth state, and redirects.
   */
  async function logout(redirect = true) {
    try {
      await authService.logout()
    } catch (err) {
      console.warn('Logout API call failed, but logging out client-side anyway.', err)
    }

    // Reset all state
    user.value = null
    localStorage.removeItem('dailyCardsByDate')
    localStorage.removeItem('dailyCompletedCards')

    // Redirect to the login page only if the current route requires authentication.
    if (redirect) {
      const requiresAuth = router.currentRoute.value.matched.some(
        (record) => record.meta.requiresAuth,
      )
      if (requiresAuth) {
        console.warn('[logout function] redirect to /login')
        await router.push({ name: 'login' })
      }
    }
  }

  // --- RETURN ---
  return {
    user,
    status,
    isAuthenticated,
    isAdmin,
    register,
    login,
    logout,
    fetchCurrentUser,
    refreshTokens,
    onAuthenticated,
  }
})
