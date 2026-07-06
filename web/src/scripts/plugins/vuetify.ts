import { createVuetify } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import themeJson from '@/assets/themes/etna-theme.json'
import { transformToVuetifyTheme } from '@/scripts/stores/themeStore.ts'
import type { Theme } from '@/scripts/types/themeColor.ts'
import { Tooltip } from 'vuetify/directives'
import { useStorage } from '@vueuse/core'
import { resolveThemeMode } from '@/scripts/themeUtils.ts'
import type { SunCache } from '@/scripts/themeUtils.ts'
import { useObjStorage } from '@/scripts/utils.ts'

const theme: Theme = themeJson
const lightDefinition = transformToVuetifyTheme(theme.schemes?.light, false)
const darkDefinition = transformToVuetifyTheme(theme.schemes?.dark, true)

function getInitialThemeMode(): 'light' | 'dark' | 'system' {
  const themeString = useStorage('themeString', 'system')
  const useSunSchedule = useStorage('useSunSchedule', true)
  const lightTime = useStorage('enableLightThemeTime', '07:00')
  const darkTime = useStorage('enableDarkThemeTime', '19:00')
  const sunData = useObjStorage<SunCache | null>('sun_cache', null)

  return resolveThemeMode(
    themeString.value,
    useSunSchedule.value,
    sunData.value?.sunrise || null,
    sunData.value?.sunset || null,
    lightTime.value,
    darkTime.value,
  )
}

export const vuetify = createVuetify({
  directives: { Tooltip },
  theme: {
    defaultTheme: getInitialThemeMode(),
    themes: {
      light: lightDefinition,
      dark: darkDefinition,
    },
  },
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: {
      mdi,
    },
  },
})
