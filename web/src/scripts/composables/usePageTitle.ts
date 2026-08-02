import { watch, onUnmounted, type MaybeRefOrGetter, toValue } from 'vue'
import { useTitleStore } from '@/scripts/stores/titleStore.ts'

export interface UseTitleOptions {
  fallback?: string
  delay?: number // Delay in ms before fallback is shown to prevent flickering
}

export function usePageTitle(
  titleSource?: MaybeRefOrGetter<string | null | undefined>,
  options: UseTitleOptions = {},
) {
  const titleStore = useTitleStore()
  const { fallback, delay = 200 } = options

  let timer: ReturnType<typeof setTimeout> | null = null

  const clearTimer = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  const updateTitle = (val: string | null | undefined) => {
    clearTimer()
    const cleanValue = val?.trim()

    if (cleanValue) {
      titleStore.setPageTitle(cleanValue)
    } else if (fallback) {
      if (delay <= 0) {
        titleStore.setPageTitle(fallback)
      } else {
        // Wait for 'delay' ms before applying fallback to prevent flicker
        timer = setTimeout(() => {
          if (!toValue(titleSource)?.trim()) {
            titleStore.setPageTitle(fallback)
          }
        }, delay)
      }
    }
  }

  if (titleSource !== undefined) {
    watch(
      () => toValue(titleSource),
      (newVal) => {
        updateTitle(newVal)
      },
      { immediate: true },
    )
  } else if (fallback) {
    titleStore.setPageTitle(fallback)
  }

  onUnmounted(() => {
    clearTimer()
  })
}

export function useDetailTitle(
  titleSource?: MaybeRefOrGetter<string | null | undefined>,
  options: UseTitleOptions = {},
) {
  const titleStore = useTitleStore()
  const { fallback, delay = 200 } = options

  let timer: ReturnType<typeof setTimeout> | null = null

  const clearTimer = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  const updateTitle = (val: string | null | undefined) => {
    clearTimer()
    const cleanValue = val?.trim()

    if (cleanValue) {
      titleStore.setDetailTitle(cleanValue)
    } else if (fallback) {
      if (delay <= 0) {
        titleStore.setDetailTitle(fallback)
      } else {
        timer = setTimeout(() => {
          if (!toValue(titleSource)?.trim()) {
            titleStore.setDetailTitle(fallback)
          }
        }, delay)
      }
    }
  }

  if (titleSource !== undefined) {
    watch(
      () => toValue(titleSource),
      (newVal) => {
        updateTitle(newVal)
      },
      { immediate: true },
    )
  } else if (fallback) {
    titleStore.setDetailTitle(fallback)
  }

  onUnmounted(() => {
    clearTimer()
    titleStore.setDetailTitle(null)
  })
}
