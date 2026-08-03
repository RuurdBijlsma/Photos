import { ref, watch, onUnmounted, toValue, type Ref, type MaybeRefOrGetter } from 'vue'

/**
 * Returns a Ref<boolean> that tracks a boolean source.
 * When the source becomes `true`, the returned ref delays changing to `true` by `delayMs`.
 * When the source becomes `false`, the returned ref changes to `false` immediately.
 *
 * @param source A ref, computed, or getter function returning a boolean
 * @param delayMs Delay in milliseconds before setting output to true (default: 150ms)
 */
export function useDelayedBoolean(source: MaybeRefOrGetter<boolean>, delayMs = 150): Ref<boolean> {
  const delayedRef = ref(false)
  let timer: ReturnType<typeof setTimeout> | null = null

  const clearTimer = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  watch(
    () => toValue(source),
    (isTrue) => {
      clearTimer()

      if (isTrue) {
        // Switched to true: start timer before showing loading state
        timer = setTimeout(() => {
          delayedRef.value = true
          timer = null
        }, delayMs)
      } else {
        // Switched to false: instantly hide loading state
        delayedRef.value = false
      }
    },
    { immediate: true },
  )

  // Clean up timer if component unmounts while request is pending
  onUnmounted(() => {
    clearTimer()
  })

  return delayedRef
}
