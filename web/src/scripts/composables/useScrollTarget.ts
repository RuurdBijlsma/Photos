import { onMounted, onUnmounted } from 'vue'
import { useLayoutStore } from '@/scripts/stores/layoutStore.ts'

export function useScrollTarget() {
  const layoutStore = useLayoutStore()
  const scrollId = `scroll-${Math.floor(Math.random() * 10000000000000).toString(36)}`

  onMounted(() => {
    layoutStore.scrollTarget = scrollId
  })
  onUnmounted(() => {
    layoutStore.scrollTarget = undefined
  })

  return { scrollId }
}
