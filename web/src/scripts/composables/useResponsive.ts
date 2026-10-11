// src/scripts/composables/useResponsive.ts
import { useDisplay } from 'vuetify/framework'
import { computed, watch } from 'vue'

/**
 * 'sm' threshold ends at 959.99px (under 'md').
 * - Mobile:  < 960px
 * - Desktop: >= 960px
 */
export const MOBILE_BREAKPOINT = 960
export const MOBILE_BREAKPOINT_PX = `${MOBILE_BREAKPOINT}px`
export const MOBILE_MAX_WIDTH_QUERY = `(max-width: ${MOBILE_BREAKPOINT - 0.01}px)` // max-width: 959.99px

// Detect touch-primary devices (phones, tablets) vs mouse-primary (desktops, laptops with touchscreens)
const hasTouchSupport = 'ontouchstart' in window || navigator.maxTouchPoints > 0
const isPrimaryInputCoarse = window.matchMedia('(pointer: coarse)').matches

export function useResponsive() {
  const display = useDisplay()

  // smAndDown: true when width < 960px
  const isMobile = display.smAndDown
  const isDesktop = display.mdAndUp

  // True on devices where touch is the primary input (phones, tablets)
  // False on laptops with touchscreens (primary input is mouse/trackpad)
  const useTouchControls = computed(() => hasTouchSupport && isPrimaryInputCoarse)

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
    useTouchControls,
    // Raw values if needed
    width: display.width,
    height: display.height,
  }
}
