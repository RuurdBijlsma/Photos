import { computed, ref } from 'vue'
import { useEventListener, useIntervalFn } from '@vueuse/core'

export function useUiHider(maxHideSeconds: number, preventUiHide: () => boolean = () => false) {
  const hideSeconds = ref(maxHideSeconds)
  const showUI = computed(() => hideSeconds.value > 0)

  const { pause, resume, isActive } = useIntervalFn(() => {
    hideSeconds.value--
    if (preventUiHide()) {
      hideSeconds.value = maxHideSeconds
    }
  }, 1000)

  useEventListener(document, 'mousemove', () => {
    hideSeconds.value = maxHideSeconds
  })
  useEventListener(document, 'click', () => {
    hideSeconds.value = maxHideSeconds
  })
  useEventListener(document, 'mouseleave', () => {
    if (!preventUiHide()) {
      hideSeconds.value = 1
    }
  })

  return { showUI, pause, resume, isActive }
}
