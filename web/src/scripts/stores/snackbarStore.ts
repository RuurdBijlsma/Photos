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
}

/** Input options when creating a snackbar */
export type SnackOptions = {
  message: string
  color?: Snack['color']
  timeout?: number
  action?: SnackAction
  icon?: string
  error?: unknown
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
      clearTimeout(snackQueue.value[index]!.timerId)
      snackQueue.value.splice(index, 1)
    }
  }

  /**
   * Adds a snackbar to the queue.
   */
  function enqueue(options: SnackOptions) {
    const id = crypto.randomUUID()
    const defaultTimeout = 10000

    const snack: Snack = {
      id,
      message: options.message,
      color: options.color || 'surface-variant', // Changed from 'white' to adapt dynamically to light/dark themes
      timeout: options.timeout ?? defaultTimeout,
      action: options.action,
      icon: options.icon,
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
    if (snack && !snack.timerId) {
      // Give it a bit more time if the user was reading it
      const remaining = Math.max(snack.timeout / 2, 2000)
      snack.timerId = setTimeout(() => {
        remove(snack.id)
      }, remaining)
    }
  }

  // --- Convenience Helpers ---

  function info(message: string, action?: SnackAction) {
    // Changed color from 'white' to 'info' to ensure dynamic, high-contrast theme compliance
    enqueue({ message, color: 'info', icon: 'mdi-information-outline', action })
  }

  function success(message: string, action?: SnackAction) {
    enqueue({ message, color: 'success', icon: 'mdi-check', action })
  }

  function warning(message: string, action?: SnackAction) {
    enqueue({ message, color: 'warning', icon: 'mdi-alert', action })
  }

  function error(message: string, error?: unknown, action?: SnackAction) {
    enqueue({
      message,
      error,
      color: 'error',
      icon: 'mdi-fire-alert',
      timeout: 10000,
      action,
    })
  }

  return {
    snackQueue,
    remove,
    enqueue,
    info,
    success,
    warning,
    error,
    pauseTimeout,
    resumeTimeout,
  }
})
