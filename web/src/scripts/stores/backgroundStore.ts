import { ref, shallowRef, watch } from 'vue'
import { defineStore } from 'pinia'
import { useThemeStore } from '@/scripts/stores/themeStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import type { Theme } from '@/scripts/types/themeColor.ts'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import type { ThemeVariant } from '@/scripts/constants.ts'
import { useThrottleFn } from '@vueuse/core'
import type { RandomPhotoResponse } from '@/scripts/types/api/photos.ts'

const BG_CACHE_KEY = 'cachedBackgroundData'
export const DEFAULT_BG_URL = '/img/etna.jpg'

type CachedBackgroundData = {
  url: string
  theme: Theme
  variant: ThemeVariant
  contrast: number
}

export const useBackgroundStore = defineStore('background', () => {
  const backgroundUrl = ref(DEFAULT_BG_URL)
  const hasFetchedForThisSession = ref(false)
  const backgroundTheme = shallowRef<Theme | null>(null)
  const colorTheme = shallowRef<Theme | null>(null)

  const themeStore = useThemeStore()
  const authStore = useAuthStore()
  const settings = useSettingStore()
  const snackbarStore = useSnackbarsStore()

  function getCachedBackgroundData() {
    const localItem = localStorage.getItem(BG_CACHE_KEY)
    if (localItem === null) return null

    try {
      return JSON.parse(localItem) as CachedBackgroundData
    } catch {
      localStorage.removeItem(BG_CACHE_KEY)
      return null
    }
  }

  async function fetchAndCacheNextBackground(force = false) {
    if (hasFetchedForThisSession.value && !force) return
    hasFetchedForThisSession.value = true

    try {
      const variant = settings.customThemeVariant
      const contrast = settings.customThemeContrast
      const response = await mediaItemService.getRandomPhoto(variant, contrast)
      const photo = response.data
      if (photo === null) {
        console.warn('getRandomPhoto returned null, probably no photos in DB with color data.')
        return
      }

      const newBgUrl = mediaItemService.getPhotoThumbnail(photo.mediaId, 1080, false)
      const newTheme = photo.theme

      if (newTheme) {
        const newCachedData: CachedBackgroundData = {
          url: newBgUrl,
          theme: newTheme,
          variant,
          contrast,
        }
        localStorage.setItem(BG_CACHE_KEY, JSON.stringify(newCachedData))
        console.log('Fetched and cached new background object for next session.')
      } else {
        console.warn('NO theme in the random photo result, not using it.')
      }
    } catch (error) {
      console.error('Failed to fetch and cache next background:', error)
    }
  }

  async function fetchColorTheme(
    color = settings.customThemeColor,
    variant = settings.customThemeVariant,
    contrast = settings.customThemeContrast ?? 0.0,
  ) {
    if (
      colorTheme.value &&
      colorTheme.value.source_color === color &&
      colorTheme.value.variant === variant &&
      colorTheme.value.contrast_level === contrast
    ) {
      return colorTheme.value
    }

    try {
      const { data } = await mediaItemService.getTheme(color, variant, contrast)
      colorTheme.value = data
      return data
    } catch (e) {
      snackbarStore.error('Could not get theme from color', e)
    }

    return null
  }

  async function refreshCurrentBackgroundTheme() {
    const sourceColor = backgroundTheme.value?.source_color
    if (!sourceColor) {
      return
    }

    try {
      const { data } = await mediaItemService.getTheme(
        sourceColor,
        settings.customThemeVariant,
        settings.customThemeContrast,
      )
      backgroundTheme.value = data
      localStorage.setItem(
        BG_CACHE_KEY,
        JSON.stringify({
          url: backgroundUrl.value,
          theme: data,
          variant: settings.customThemeVariant,
          contrast: settings.customThemeContrast,
        } satisfies CachedBackgroundData),
      )
      themeStore.setThemesFromJson(data)
      return data
    } catch (e) {
      snackbarStore.error('Could not update background theme variant or contrast', e)
      return
    }
  }

  async function newBackgroundTheme() {
    let newBgJson: RandomPhotoResponse | null = null
    try {
      const { data } = await mediaItemService.getRandomPhoto(
        settings.customThemeVariant,
        settings.customThemeContrast,
      )
      newBgJson = data
    } catch (e) {
      snackbarStore.error('Could not get new background theme', e)
    }
    if (!newBgJson) return
    backgroundTheme.value = newBgJson.theme
    backgroundUrl.value = mediaItemService.getPhotoThumbnail(newBgJson.mediaId, 1080, false)
    themeStore.setThemesFromJson(backgroundTheme.value)
  }

  function getBackgroundTheme() {
    if (backgroundTheme.value) return backgroundTheme.value

    const data = getCachedBackgroundData()
    if (data !== null) {
      if (
        'variant' in data &&
        data.variant === settings.customThemeVariant &&
        (!('contrast' in data) || data.contrast === settings.customThemeContrast)
      ) {
        backgroundTheme.value = data.theme
        backgroundUrl.value = data.url
        return data.theme
      }
    }

    backgroundUrl.value = DEFAULT_BG_URL
    return null
  }

  async function setCorrectTheme() {
    const theme = settings.useImageBackground ? getBackgroundTheme() : await fetchColorTheme()
    if (theme) themeStore.setThemesFromJson(theme)
  }

  // --- WATCHERS ---

  watch(
    () => settings.customThemeColor,
    () => setCorrectTheme(),
  )

  watch([() => settings.customThemeVariant, () => settings.customThemeContrast], async () => {
    if (settings.useImageBackground) {
      await refreshCurrentBackgroundTheme()
    } else {
      await setCorrectTheme()
    }
    if (settings.useImageBackground && authStore.isAuthenticated) {
      fetchAndCacheNextBackground(true)
    }
  })

  const throttledTheme = useThrottleFn(setCorrectTheme, 50, true, true)

  watch(
    () => settings.useImageBackground,
    () => {
      return throttledTheme()
    },
  )

  function initialize() {
    setCorrectTheme().then()

    requestIdleCallback(() => {
      if (authStore.isAuthenticated) {
        fetchAndCacheNextBackground()
      }

      watch(
        () => authStore.isAuthenticated,
        (isNowAuthenticated) => {
          if (isNowAuthenticated) {
            fetchAndCacheNextBackground()
          }
        },
      )
    })
  }

  return {
    backgroundUrl,
    initialize,
    backgroundTheme,
    colorTheme,
    newBackgroundTheme,
  }
})
