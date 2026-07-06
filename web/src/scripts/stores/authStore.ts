import { computed, ref, type Ref, watch } from 'vue'
import { defineStore } from 'pinia'
import authService from '@/scripts/services/authService.ts'
import type { CreateUser, LoginUser, Tokens, User } from '@/scripts/types/api/auth.ts'
import { useRouter } from 'vue-router'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { useIntervalFn, useStorage } from '@vueuse/core'
import { useObjStorage } from '@/scripts/utils.ts'

type AuthStatus = 'idle' | 'loading' | 'error' | 'success'

export const useAuthStore = defineStore('auth', () => {
  const systemStore = useSystemStore()

  // --- STATE ---
  const user = useObjStorage<User | null>('authUser', null)
  const accessToken = useStorage<string | null>('accessToken', null)
  const refreshToken = useStorage<string | null>('refreshToken', null)
  const expiry = useStorage<number | null>('expiry', null)
  const status: Ref<AuthStatus> = ref('idle')
  const router = useRouter()

  // --- GETTERS ---
  const isAuthenticated = computed(() => !!accessToken.value && !!user.value)
  const isAdmin = computed(() => user.value?.role === 'admin')

  // --- ACTIONS ---

  /**
   * Internal helper to manage token state and persistence.
   */
  function setTokens(newAccessToken: string, newRefreshToken: string, newExpiry: number) {
    accessToken.value = newAccessToken
    refreshToken.value = newRefreshToken
    expiry.value = newExpiry
  }

  /**
   * Refreshes the access token using the refresh token.
   * If successful, it updates the tokens in the store and returns them.
   * If it fails, it will trigger a logout and throw an error.
   */
  async function refreshTokens(): Promise<Tokens> {
    if (!refreshToken.value) {
      console.warn('[no refresh token available] call logout()')
      await logout(false)
      throw new Error('No refresh token available.')
    }
    try {
      const response = await authService.refreshSession({ refreshToken: refreshToken.value })
      const {
        accessToken: newAccessToken,
        refreshToken: newRefreshToken,
        expiry: newExpiry,
      } = response.data
      setTokens(newAccessToken, newRefreshToken, newExpiry)

      requestIdleCallback(fetchCurrentUser)

      return response.data
    } catch (error) {
      // If refresh fails, log the user out completely
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
    () => isAuthenticated,
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
    // Configured background polling via useIntervalFn
  }

  /**
   * Fetches the current user's data using the access token.
   */
  async function fetchCurrentUser() {
    if (!accessToken.value) return
    const response = await authService.getMe()
    user.value = response.data
  }

  /**
   * Logs the user in, fetches tokens, and gets user data.
   */
  async function login(credentials: LoginUser) {
    status.value = 'loading'
    try {
      const response = await authService.login(credentials)
      setTokens(response.data.accessToken, response.data.refreshToken, response.data.expiry)
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
    if (refreshToken.value) {
      try {
        await authService.logout({ refreshToken: refreshToken.value })
      } catch (err) {
        console.warn('Logout API call failed, but logging out client-side anyway.', err)
      }
    }

    // Reset all state
    user.value = null
    accessToken.value = null
    refreshToken.value = null
    expiry.value = null
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
    accessToken,
    refreshToken,
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
