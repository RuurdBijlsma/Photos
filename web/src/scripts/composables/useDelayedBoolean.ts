import { ref, watch, toValue, type Ref, type MaybeRefOrGetter } from 'vue'
import { useTimeoutFn } from '@vueuse/core'

export interface DelayOptions {
  /** Delay before turning true (ms). Default: 150 */
  delayOn?: number
  /** Delay before turning false (ms). Default: 0 */
  delayOff?: number
}

export function useDelayedBoolean(
  source: MaybeRefOrGetter<boolean>,
  options: number | DelayOptions = 150,
): Ref<boolean> {
  const delayOn = typeof options === 'number' ? options : (options.delayOn ?? 150)
  const delayOff = typeof options === 'number' ? 0 : (options.delayOff ?? 0)

  const delayedRef = ref(false)

  const { start: startOn, stop: stopOn } = useTimeoutFn(() => (delayedRef.value = true), delayOn, {
    immediate: false,
  })

  const { start: startOff, stop: stopOff } = useTimeoutFn(
    () => (delayedRef.value = false),
    delayOff,
    { immediate: false },
  )

  watch(
    () => toValue(source),
    (isTrue) => {
      stopOn()
      stopOff()

      if (isTrue) {
        startOn()
      } else if (delayOff > 0) {
        startOff()
      } else {
        delayedRef.value = false
      }
    },
    { immediate: true },
  )

  return delayedRef
}
