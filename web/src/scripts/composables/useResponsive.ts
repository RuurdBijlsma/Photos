// src/scripts/composables/useResponsive.ts
import { useDisplay } from 'vuetify/framework'
import { watch } from 'vue'

/**
 * 'sm' threshold ends at 959.99px (under 'md').
 * - Mobile:  < 960px
 * - Desktop: >= 960px
 */
export const MOBILE_BREAKPOINT = 960
export const MOBILE_BREAKPOINT_PX = `${MOBILE_BREAKPOINT}px`
export const MOBILE_MAX_WIDTH_QUERY = `(max-width: ${MOBILE_BREAKPOINT - 0.01}px)` // max-width: 959.99px

export function useResponsive() {
  const display = useDisplay()

  // smAndDown: true when width < 960px
  const isMobile = display.smAndDown
  const isDesktop = display.mdAndUp

  watch(
    isMobile,
    () => {
      console.log('IS MOBILE', isMobile.value)
    },
    { immediate: true },
  )

  return {
    isMobile,
    isDesktop,
    // Raw values if needed
    width: display.width,
    height: display.height,
  }
}
