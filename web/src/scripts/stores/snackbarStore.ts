import { defineStore } from 'pinia'
import { ref } from 'vue'
import { isAxiosError } from 'axios'

// --- Types ---

export interface SnackAction {
  label: string
  onClick: () => unknown
  hideOnClick?: boolean
}

export interface Snack {
  id: string
  message: string
  icon?: string
  color: 'success' | 'info' | 'warning' | 'error' | 'surface-variant' | string
  timeout: number
  action?: SnackAction
  error?: Error
  errorData?: { error: string }
  timerId?: ReturnType<typeof setTimeout>
  loading?: boolean
  dismissable?: boolean
}

/** Input options when creating a snackbar */
export type SnackOptions = {
  message: string
  color?: Snack['color']
  timeout?: number
  action?: SnackAction
  icon?: string
  error?: unknown
  loading?: boolean
  dismissable?: boolean
}

// --- Store ---

export const useSnackbarsStore = defineStore('snackbars', () => {
  const snackQueue = ref<Snack[]>([])

  /**
   * Removes a snackbar by ID.
   */
  function remove(id: string) {
    const index = snackQueue.value.findIndex((s) => s.id === id)
    if (index > -1) {
      if (snackQueue.value[index]!.timerId) {
        clearTimeout(snackQueue.value[index]!.timerId)
      }
      snackQueue.value.splice(index, 1)
    }
  }

  /**
   * Adds a snackbar to the queue and returns its ID.
   */
  function enqueue(options: SnackOptions): string {
    const id = crypto.randomUUID()
    const defaultTimeout = 10000

    const snack: Snack = {
      id,
      message: options.message,
      color: options.color || 'surface-variant',
      timeout: options.timeout ?? defaultTimeout,
      action: options.action,
      icon: options.icon,
      loading: options.loading ?? false,
      dismissable: options.dismissable ?? true,
    }

    // Process Error objects if present
    if (options.error) {
      if (isAxiosError(options.error)) {
        snack.error = options.error
        snack.errorData = options.error.response?.data
        console.error('[Snack Axios]', options.message, options.error.response?.data)
      } else if (options.error instanceof Error) {
        snack.error = options.error
        console.error('[Snack Error]', options.message, options.error)
      } else {
        snack.error = new Error(String(options.error))
        console.error('[Snack Unknown]', options.message, options.error)
      }
      if (!options.color) snack.color = 'error'
    }

    snackQueue.value.push(snack)

    // Start timer
    startTimer(snack)

    return id
  }

  /**
   * Updates an existing snackbar's properties by ID.
   */
  function update(id: string, updates: Partial<SnackOptions>) {
    const index = snackQueue.value.findIndex((s) => s.id === id)
    if (index === -1) return

    const snack = snackQueue.value[index]!

    if (snack.timerId) {
      clearTimeout(snack.timerId)
      snack.timerId = undefined
    }

    if (updates.message !== undefined) snack.message = updates.message
    if (updates.color !== undefined) snack.color = updates.color
    if (updates.timeout !== undefined) snack.timeout = updates.timeout
    if (updates.action !== undefined) snack.action = updates.action
    if (updates.icon !== undefined) snack.icon = updates.icon
    if (updates.loading !== undefined) snack.loading = updates.loading
    if (updates.dismissable !== undefined) snack.dismissable = updates.dismissable

    // Handle updating error payloads
    if (updates.error !== undefined) {
      if (isAxiosError(updates.error)) {
        snack.error = updates.error
        snack.errorData = updates.error.response?.data
      } else if (updates.error instanceof Error) {
        snack.error = updates.error
      } else if (updates.error) {
        snack.error = new Error(String(updates.error))
      } else {
        snack.error = undefined
        snack.errorData = undefined
      }
      if (!updates.color) snack.color = 'error'
    }

    startTimer(snack)
  }

  function startTimer(snack: Snack) {
    if (snack.timeout > 0) {
      snack.timerId = setTimeout(() => {
        remove(snack.id)
      }, snack.timeout)
    }
  }

  /**
   * Pauses the auto-close timer (e.g., on hover).
   */
  function pauseTimeout(id: string) {
    const snack = snackQueue.value.find((s) => s.id === id)
    if (snack && snack.timerId) {
      console.log('TIMER PAUSED')
      clearTimeout(snack.timerId)
      snack.timerId = undefined
    }
  }

  /**
   * Resumes the auto-close timer (e.g., on mouse leave).
   */
  function resumeTimeout(id: string) {
    const snack = snackQueue.value.find((s) => s.id === id)
    // Only resume if the snackbar actually has a standard expiring timeout
    if (snack && snack.timeout > 0 && !snack.timerId) {
      const remaining = Math.max(snack.timeout / 2, 2000)
      snack.timerId = setTimeout(() => {
        remove(snack.id)
      }, remaining)
    }
  }

  // --- Convenience Helpers ---

  function info(message: string, action?: SnackAction): string {
    return enqueue({ message, color: 'info', icon: 'mdi-information-outline', action })
  }

  function success(message: string, action?: SnackAction): string {
    return enqueue({ message, color: 'success', icon: 'mdi-check', action })
  }

  function warning(message: string, action?: SnackAction): string {
    return enqueue({ message, color: 'warning', icon: 'mdi-alert', action })
  }

  function error(message: string, error?: unknown, action?: SnackAction): string {
    return enqueue({
      message,
      error,
      color: 'error',
      icon: 'mdi-fire-alert',
      timeout: 10000,
      action,
    })
  }

  /**
   * Spawns a persistent, loading snackbar that is non-dismissable by default.
   */
  function loading(message: string, action?: SnackAction): string {
    return enqueue({
      message,
      color: 'info',
      timeout: 0,
      dismissable: false,
      loading: true,
      action,
    })
  }

  return {
    snackQueue,
    remove,
    enqueue,
    update,
    info,
    success,
    warning,
    error,
    loading,
    pauseTimeout,
    resumeTimeout,
  }
})
