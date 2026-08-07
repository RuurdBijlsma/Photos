import { createVuetify } from 'vuetify'
import themeJson from '@/assets/themes/etna-theme.json'
import { transformToVuetifyTheme } from '@/scripts/stores/themeStore.ts'
import type { Theme } from '@/scripts/types/themeColor.ts'
import { Tooltip } from 'vuetify/directives'
import { useStorage } from '@vueuse/core'
import { resolveThemeMode } from '@/scripts/themeUtils.ts'
import type { SunCache } from '@/scripts/themeUtils.ts'
import { useObjStorage } from '@/scripts/utils.ts'

import MdiChevronDown from '~icons/mdi/chevron-down'
import MdiCheckboxMarked from '~icons/mdi/checkbox-marked'
import MdiCheckboxBlankOutline from '~icons/mdi/checkbox-blank-outline'
import MdiClose from '~icons/mdi/close'
import MdiCloseCircle from '~icons/mdi/close-circle'
import MdiMinus from '~icons/mdi/minus'
import MdiInformation from '~icons/mdi/information'
import MdiChevronLeft from '~icons/mdi/chevron-left'
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiRadioboxBlank from '~icons/mdi/radiobox-blank'
import MdiRadioboxMarked from '~icons/mdi/radiobox-marked'
import MdiStar from '~icons/mdi/star'
import MdiStarHalf from '~icons/mdi/star-half-full'
import MdiStarOutline from '~icons/mdi/star-outline'

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
    defaultSet: 'component',
    aliases: {
      cancel: MdiCloseCircle,
      checkboxOff: MdiCheckboxBlankOutline,
      checkboxOn: MdiCheckboxMarked,
      checkboxIndeterminate: MdiMinus,
      clear: MdiClose,
      close: MdiClose,
      complete: MdiCheckboxMarked,
      dropdown: MdiChevronDown,
      edit: MdiClose,
      error: MdiCloseCircle,
      info: MdiInformation,
      next: MdiChevronRight,
      prev: MdiChevronLeft,
      radioOff: MdiRadioboxBlank,
      radioOn: MdiRadioboxMarked,
      ratingEmpty: MdiStarOutline,
      ratingFull: MdiStar,
      ratingHalf: MdiStarHalf,
      subgroup: MdiChevronDown,
      unfold: MdiChevronDown,
    },
  },
})
