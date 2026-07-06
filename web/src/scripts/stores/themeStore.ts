import { type Ref, ref, watch, onBeforeUnmount } from 'vue'
import { defineStore } from 'pinia'
import type { ThemeDefinition } from 'vuetify'
import type { DynamicScheme, Palette, Theme } from '@/scripts/types/themeColor.ts'
import { useTheme } from 'vuetify/framework'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useSunStore } from '@/scripts/stores/sunStore.ts'
import { resolveThemeMode } from '@/scripts/themeUtils.ts'

type VuetifyColors = ThemeDefinition['colors']

export function generateColorVariations(
  colors: VuetifyColors,
  name: string,
  palette: Palette,
  isDark: boolean,
) {
  if (!colors) return

  if (isDark) {
    // Dark Theme: Guess that base color is a high tone (e.g., 80).
    colors[`${name}-lighten-1`] = palette.tones['90']
    colors[`${name}-darken-1`] = palette.tones['70']
    colors[`${name}-darken-2`] = palette.tones['60']
    colors[`${name}-darken-3`] = palette.tones['50']
  } else {
    // Light Theme: Guess that base color is a mid-tone (e.g., 40).
    colors[`${name}-lighten-1`] = palette.tones['50']
    colors[`${name}-lighten-2`] = palette.tones['60']
    colors[`${name}-lighten-3`] = palette.tones['70']
    colors[`${name}-darken-1`] = palette.tones['30']
    colors[`${name}-darken-2`] = palette.tones['20']
    colors[`${name}-darken-3`] = palette.tones['10']
  }
}

/**
 * Transforms mcu scheme into a Vuetify ThemeDefinition object.
 * @param {DynamicScheme} scheme - The light or dark scheme from your types.
 * @param isDark is dark
 * @returns {ThemeDefinition} A complete Vuetify theme definition.
 */
export function transformToVuetifyTheme(scheme: DynamicScheme, isDark: boolean): ThemeDefinition {
  const colors: VuetifyColors = {
    background: scheme.background,
    surface: scheme.surface,
    primary: scheme.primary,
    secondary: scheme.secondary,
    tertiary: scheme.tertiary,
    'on-background': scheme.on_surface,
    'on-surface': scheme.on_surface,
    'on-primary': scheme.on_primary,
    'on-secondary': scheme.on_secondary,
    'on-tertiary': scheme.on_tertiary,
    'surface-variant': scheme.surface_variant,
    'on-surface-variant': scheme.on_surface_variant,
    'primary-container': scheme.primary_container,
    'on-primary-container': scheme.on_primary_container,
    'secondary-container': scheme.secondary_container,
    'on-secondary-container': scheme.on_secondary_container,
    'tertiary-container': scheme.tertiary_container,
    'on-tertiary-container': scheme.on_tertiary_container,
    'error-container': scheme.error_container,
    'on-error-container': scheme.on_error_container,
    outline: scheme.outline,
    'outline-variant': scheme.outline_variant,
    scrim: scheme.scrim,
    shadow: scheme.shadow,
    'surface-container-highest': scheme.surface_container_highest,
    'surface-container-high': scheme.surface_container_high,
    'surface-container': scheme.surface_container,
    'surface-container-low': scheme.surface_container_low,
    'surface-container-lowest': scheme.surface_container_lowest,

    error: scheme.error,
    'on-error': scheme.on_error,
    info: isDark ? '#64B5F6' : '#2196F3',
    'on-info': isDark ? '#0f1b25' : '#d3e6f5',
    success: isDark ? '#81C784' : '#2E7D32',
    'on-success': isDark ? '#203121' : '#e0f3e1',
    warning: isDark ? '#FFD54F' : '#F9A825',
    'on-warning': isDark ? '#211c0a' : '#1c1304',

    'surface-bright': scheme.surface_bright,
    'surface-light': scheme.surface_tint,
  }

  generateColorVariations(colors, 'primary', scheme.primary_palette, scheme.is_dark)
  generateColorVariations(colors, 'secondary', scheme.secondary_palette, scheme.is_dark)
  generateColorVariations(colors, 'tertiary', scheme.tertiary_palette, scheme.is_dark)

  return {
    dark: scheme.is_dark,
    colors,
    variables: {
      'border-color': scheme.outline,
      'border-opacity': 0.12,
      'high-emphasis-opacity': 0.87,
      'medium-emphasis-opacity': 0.6,
      'disabled-opacity': 0.38,
      'idle-opacity': 0.04,
      'hover-opacity': 0.04,
      'focus-opacity': 0.12,
      'selected-opacity': 0.08,
      'activated-opacity': 0.12,
      'pressed-opacity': 0.12,
      'dragged-opacity': 0.08,
      'theme-kbd': scheme.on_surface,
      'theme-on-kbd': scheme.on_surface_variant,
      'theme-code': scheme.surface,
      'theme-on-code': scheme.on_surface_variant,
    },
  }
}

export const useThemeStore = defineStore('theme', () => {
  // --- STATE ---
  const currentTheme: Ref<{ light: ThemeDefinition; dark: ThemeDefinition } | null> = ref(null)
  const vuetifyTheme = useTheme()

  const settings = useSettingStore()
  const sun = useSunStore()
  let isInitialized = false
  let timeoutId: ReturnType<typeof setTimeout> | null = null

  // --- ACTIONS ---

  function themeFromJson(
    themeData: Theme,
  ): null | { light: ThemeDefinition; dark: ThemeDefinition } {
    if (!themeData) {
      console.warn('setThemesFromJson called with empty or invalid theme data.')
      return null
    }
    const { schemes } = themeData

    const lightTheme = transformToVuetifyTheme(schemes.light, false)
    const darkTheme = transformToVuetifyTheme(schemes.dark, true)

    return { light: lightTheme, dark: darkTheme }
  }

  function setThemesFromJson(themeData: Theme) {
    const theme = themeFromJson(themeData)
    currentTheme.value = theme
    if (vuetifyTheme.themes.value.dark && theme?.dark.colors) {
      //@ts-expect-error Error
      vuetifyTheme.themes.value.dark.colors = theme.dark.colors
    }
    if (vuetifyTheme.themes.value.light && theme?.light.colors) {
      //@ts-expect-error Error
      vuetifyTheme.themes.value.light.colors = theme.light.colors
    }
  }

  /**
   * Plans a setTimeout execution targeting the next upcoming transition point (sunset or sunrise)
   */
  function scheduleNextTransition() {
    if (timeoutId) {
      clearTimeout(timeoutId)
      timeoutId = null
    }

    if (settings.themeString !== 'schedule') {
      return
    }

    const now = new Date()
    let nextTransition: Date | null = null

    if (settings.useSunSchedule) {
      const sunrise = sun.sunrise
      const sunset = sun.sunset

      if (sunrise && sunset) {
        const nextSunrise = new Date(sunrise)
        nextSunrise.setFullYear(now.getFullYear(), now.getMonth(), now.getDate())
        if (nextSunrise <= now) {
          nextSunrise.setDate(nextSunrise.getDate() + 1)
        }

        const nextSunset = new Date(sunset)
        nextSunset.setFullYear(now.getFullYear(), now.getMonth(), now.getDate())
        if (nextSunset <= now) {
          nextSunset.setDate(nextSunset.getDate() + 1)
        }

        nextTransition = nextSunrise < nextSunset ? nextSunrise : nextSunset
      } else {
        timeoutId = setTimeout(() => {
          updateActiveTheme()
        }, 5000)
        return
      }
    } else {
      const lightTime = settings.enableLightThemeTime || '07:00'
      const darkTime = settings.enableDarkThemeTime || '19:00'

      const getNextTransitionDate = (timeStr: string) => {
        const [h, m] = timeStr.split(':').map(Number)
        const d = new Date()
        d.setHours(h, m, 0, 0)
        if (d <= now) {
          d.setDate(d.getDate() + 1)
        }
        return d
      }

      const nextLight = getNextTransitionDate(lightTime)
      const nextDark = getNextTransitionDate(darkTime)
      nextTransition = nextLight < nextDark ? nextLight : nextDark
    }

    if (nextTransition) {
      const delay = nextTransition.getTime() - now.getTime()
      const safeDelay = Math.max(delay, 1000)

      timeoutId = setTimeout(() => {
        updateActiveTheme()
      }, safeDelay)
    }
  }

  function updateActiveTheme() {
    const targetTheme = resolveThemeMode(
      settings.themeString,
      settings.useSunSchedule,
      sun.sunrise,
      sun.sunset,
      settings.enableLightThemeTime,
      settings.enableDarkThemeTime,
    )

    if (vuetifyTheme.global.name.value !== targetTheme) {
      vuetifyTheme.change(targetTheme)
    }

    scheduleNextTransition()
  }

  function initThemeSync() {
    if (isInitialized) return
    isInitialized = true

    sun.fetchSunTimes(true)

    watch(
      [
        () => settings.themeString,
        () => settings.useSunSchedule,
        () => settings.enableLightThemeTime,
        () => settings.enableDarkThemeTime,
        () => sun.sunrise,
        () => sun.sunset,
      ],
      () => {
        updateActiveTheme()
      },
      { immediate: true },
    )

    onBeforeUnmount(() => {
      if (timeoutId) {
        clearTimeout(timeoutId)
      }
    })
  }

  return {
    currentTheme,
    setThemesFromJson,
    themeFromJson,
    updateActiveTheme,
    initThemeSync,
  }
})
