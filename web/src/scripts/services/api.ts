import axios, { type AxiosError, type InternalAxiosRequestConfig } from 'axios'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || 'http://localhost:9475/',
  headers: {
    'Content-Type': 'application/json',
  },
})

// --- Request Interceptor ---
// This injects the access token into every outgoing request
apiClient.interceptors.request.use(
  (config) => {
    // We must import the store inside the function to avoid circular dependency issues
    const authStore = useAuthStore()
    const token = authStore.accessToken

    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  },
  (error: AxiosError) => Promise.reject(error),
)

// --- Response Interceptor ---

// A variable to track if a token refresh is currently in progress
let isRefreshing = false
// A queue to hold requests that failed due to an expired token
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let failedQueue: { resolve: (value: unknown) => void; reject: (reason?: any) => void }[] = []

/**
 * Processes the queue of failed requests.
 * @param error - An error if the token refresh failed, otherwise null.
 * @param token - The new access token if the refresh was successful.
 */
const processQueue = (error: Error | null, token: string | null = null) => {
  failedQueue.forEach((prom) => {
    if (error) {
      prom.reject(error)
    } else {
      prom.resolve(token)
    }
  })
  failedQueue = []
}

apiClient.interceptors.response.use(
  (response) => {
    // Any status code that lie within the range of 2xx cause this function to trigger
    return response
  },
  async (error: AxiosError) => {
    // Any status codes that falls outside the range of 2xx cause this function to trigger
    const originalRequest = error.config as InternalAxiosRequestConfig & { _retry?: boolean }

    // We only want to handle 401 errors that are not for the refresh token endpoint itself
    if (error.response?.status === 401 && originalRequest.url !== '/auth/refresh') {
      if (isRefreshing) {
        // If a refresh is already in progress, we queue the request
        return new Promise((resolve, reject) => {
          failedQueue.push({ resolve, reject })
        })
          .then((token) => {
            if (originalRequest.headers) {
              originalRequest.headers.Authorization = `Bearer ${token}`
            }
            // Retry the original request with the new token
            return apiClient(originalRequest)
          })
          .catch((err) => {
            return Promise.reject(err)
          })
      }

      originalRequest._retry = true
      isRefreshing = true

      const authStore = useAuthStore()
      try {
        const newTokens = await authStore.refreshTokens()
        if (originalRequest.headers) {
          originalRequest.headers.Authorization = `Bearer ${newTokens.accessToken}`
        }
        // Process the queue with the new token
        processQueue(null, newTokens.accessToken)
        // Retry the original request
        return apiClient(originalRequest)
      } catch (refreshError) {
        // If the refresh token call fails, process the queue with an error
        processQueue(refreshError as Error, null)
        // The refreshTokens action will have already triggered a logout
        return Promise.reject(refreshError)
      } finally {
        isRefreshing = false
      }
    }

    return Promise.reject(error)
  },
)

export default apiClient
