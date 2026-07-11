import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useEventListener } from '@vueuse/core'

export function useUiHider(maxHideSeconds: number, preventUiHide: () => boolean = () => false) {
  const hideSeconds = ref(maxHideSeconds)
  const showUI = computed(() => hideSeconds.value > 0)
  const hideTimer = setInterval(() => {
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

  onBeforeUnmount(() => clearInterval(hideTimer))
  return { showUI, hideTimer }
}
