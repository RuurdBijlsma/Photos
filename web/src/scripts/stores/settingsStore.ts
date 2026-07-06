import { defineStore } from 'pinia'
import { useStorage } from '@vueuse/core'
import type { ThemeType, ThemeVariant } from '@/scripts/constants.ts'

export const USE_IMAGE_GLOW = false
export const DARK_PHOTO_VIEWER = true
export const LIGHT_PHOTO_VIEWER_MAP = true
export const USE_BACKDROP_BLUR = true
export const TIMELINE_ROW_HEIGHT = 320
export const TIMELINE_USE_DAY_LABELS = false
export const TIMELINE_ASYNC_IMAGE_DECODING = false
export const THEME_STRING: ThemeType = 'system'
export const ENABLE_LIGHT_THEME_TIME = '07:00'
export const ENABLE_DARK_THEME_TIME = '19:00'
export const USE_SUN_SCHEDULE = false
export const USE_IMAGE_BACKGROUND = true
export const CUSTOM_THEME_COLOR = '#462de8'
export const CUSTOM_THEME_VARIANT: ThemeVariant = 'Expressive'
export const CUSTOM_THEME_CONTRAST = 0.2

export const useSettingStore = defineStore('settings', () => {
  // Theme -> Mode
  const themeString = useStorage<ThemeType>('themeString', THEME_STRING)
  const enableLightThemeTime = useStorage('enableLightThemeTime', ENABLE_LIGHT_THEME_TIME)
  const enableDarkThemeTime = useStorage('enableDarkThemeTime', ENABLE_DARK_THEME_TIME)
  const useSunSchedule = useStorage('useSunSchedule', USE_SUN_SCHEDULE)
  // Theme -> Color
  const useImageBackground = useStorage('useImageBackground', USE_IMAGE_BACKGROUND)
  const customThemeColor = useStorage('customThemeColor', CUSTOM_THEME_COLOR)
  const customThemeVariant = useStorage<ThemeVariant>('customThemeVariant', CUSTOM_THEME_VARIANT)
  const customThemeContrast = useStorage<number>('customThemeContrast', CUSTOM_THEME_CONTRAST)
  // UI -> Photo Viewer
  const useImageGlow = useStorage('imageGlow', USE_IMAGE_GLOW)
  const darkPhotoViewer = useStorage('darkPhotoViewer', DARK_PHOTO_VIEWER)
  const lightPhotoViewerMap = useStorage('lightPhotoViewerMap', LIGHT_PHOTO_VIEWER_MAP)
  const playMotionPhotos = useStorage('playMotionPhotos', true)
  // UI -> General
  const useBackdropBlur = useStorage('backdropBlur', USE_BACKDROP_BLUR)
  // UI -> Timeline
  const timelineRowHeight = useStorage('timelineRowHeight', TIMELINE_ROW_HEIGHT)
  const timelineUseDayLabels = useStorage('timelineUseDayLabels', TIMELINE_USE_DAY_LABELS)
  const asyncImageDecoding = useStorage('timelineAsyncImageDecoding', TIMELINE_ASYNC_IMAGE_DECODING)

  return {
    useImageGlow,
    useBackdropBlur,
    useImageBackground,
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
