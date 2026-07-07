import { watch } from 'vue'
import { useRefreshStore } from '@/scripts/stores/refreshStore'

interface RefreshOptions {
  immediate?: boolean
}

export function useRefreshFunction(callback: () => void, options: RefreshOptions = {}) {
  const refreshStore = useRefreshStore()

  watch(
    () => refreshStore.counter,
    () => callback(),
    {
      immediate: options.immediate ?? false,
    },
  )
}
