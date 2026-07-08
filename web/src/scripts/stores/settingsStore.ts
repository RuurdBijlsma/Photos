import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'
import type { ThemeType, ThemeVariant } from '@/scripts/constants.ts'

export const TIMELINE_ROW_HEIGHT = 320
export const CUSTOM_THEME_CONTRAST = 0.2

export const useSettingStore = defineStore('settings', () => {
  // Theme -> Mode
  const themeString = useStorage<ThemeType>('themeString', 'system')
  const enableLightThemeTime = useStorage('enableLightThemeTime', '07:00')
  const enableDarkThemeTime = useStorage('enableDarkThemeTime', '19:00')
  const useSunSchedule = useStorage('useSunSchedule', false)
  // Theme -> Color
  const useImageBackground = useStorage('useImageBackground', true)
  const randomizeBackground = useStorage('randomizeBackground', true)
  const customThemeColor = useStorage('customThemeColor', '#462de8')
  const customThemeVariant = useStorage<ThemeVariant>('customThemeVariant', 'Expressive')
  const customThemeContrast = useStorage<number>('customThemeContrast', CUSTOM_THEME_CONTRAST)
  // UI -> Photo Viewer
  const useImageGlow = useStorage('imageGlow', false)
  const darkPhotoViewer = useStorage('darkPhotoViewer', true)
  const lightPhotoViewerMap = useStorage('lightPhotoViewerMap', true)
  const playMotionPhotos = useStorage('playMotionPhotos', true)
  // UI -> General
  const useBackdropBlur = useStorage('backdropBlur', true)
  // UI -> Timeline
  const timelineRowHeight = useStorage('timelineRowHeight', TIMELINE_ROW_HEIGHT)
  const timelineUseDayLabels = useStorage('timelineUseDayLabels', false)
  const asyncImageDecoding = useStorage('timelineAsyncImageDecoding', false)

  return {
    useImageGlow,
    useBackdropBlur,
    useImageBackground,
    randomizeBackground,
    customThemeColor,
    customThemeVariant,
    customThemeContrast,
    timelineRowHeight,
    timelineUseDayLabels,
    asyncImageDecoding,
    darkPhotoViewer,
    lightPhotoViewerMap,
    playMotionPhotos,
    themeString,
    useSunSchedule,
    enableLightThemeTime,
    enableDarkThemeTime,
  }
})
