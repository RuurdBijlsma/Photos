import axios, { type AxiosError, type InternalAxiosRequestConfig } from 'axios'
import { useAuthStore } from '@/scripts/stores/authStore.ts'

const apiClient = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || 'http://localhost:9475',
  headers: {
    'Content-Type': 'application/json',
  },
  withCredentials: true,
})

// --- Response Interceptor ---
let isRefreshing = false
let failedQueue: { resolve: (value: unknown) => void; reject: (reason?: any) => void }[] = []

const processQueue = (error: Error | null) => {
  failedQueue.forEach((prom) => {
    if (error) {
      prom.reject(error)
    } else {
      prom.resolve(null)
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
          .then(() => {
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
        await authStore.refreshTokens()
        // Process the queue with the new token
        processQueue(null)
        return apiClient(originalRequest)
      } catch (refreshError) {
        // If the refresh token call fails, process the queue with an error
        processQueue(refreshError as Error)
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
