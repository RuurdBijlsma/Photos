import { createVuetify } from 'vuetify'
import themeJson from '@/assets/themes/etna-theme.json'
import { transformToVuetifyTheme } from '@/scripts/stores/themeStore.ts'
import type { Theme } from '@/scripts/types/themeColor.ts'
import { Tooltip } from 'vuetify/directives'
import { useStorage } from '@vueuse/core'
import { resolveThemeMode } from '@/scripts/themeUtils.ts'
import type { SunCache } from '@/scripts/themeUtils.ts'
import { useObjStorage } from '@/scripts/utils.ts'
import MdiAlert from '~icons/mdi/alert'
import MdiAlertCircle from '~icons/mdi/alert-circle'
import MdiArrowDown from '~icons/mdi/arrow-down'
import MdiArrowUp from '~icons/mdi/arrow-up'
import MdiCalendar from '~icons/mdi/calendar'
import MdiCheckboxBlankOutline from '~icons/mdi/checkbox-blank-outline'
import MdiCheckboxMarked from '~icons/mdi/checkbox-marked'
import MdiCheckCircle from '~icons/mdi/check-circle'
import MdiChevronDoubleLeft from '~icons/mdi/chevron-double-left'
import MdiChevronDoubleRight from '~icons/mdi/chevron-double-right'
import MdiChevronDown from '~icons/mdi/chevron-down'
import MdiChevronLeft from '~icons/mdi/chevron-left'
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiChevronUp from '~icons/mdi/chevron-up'
import MdiCircle from '~icons/mdi/circle'
import MdiClose from '~icons/mdi/close'
import MdiCloseCircle from '~icons/mdi/close-circle'
import MdiEye from '~icons/mdi/eye'
import MdiEyeOff from '~icons/mdi/eye-off'
import MdiInformation from '~icons/mdi/information'
import MdiMenu from '~icons/mdi/menu'
import MdiMinus from '~icons/mdi/minus'
import MdiPaperclip from '~icons/mdi/paperclip'
import MdiPencil from '~icons/mdi/pencil'
import MdiPlus from '~icons/mdi/plus'
import MdiRadioboxBlank from '~icons/mdi/radiobox-blank'
import MdiRadioboxMarked from '~icons/mdi/radiobox-marked'
import MdiRefresh from '~icons/mdi/refresh'
import MdiStar from '~icons/mdi/star'
import MdiStarHalfFull from '~icons/mdi/star-half-full'
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
      calendar: MdiCalendar,
      cancel: MdiCloseCircle,
      checkboxIndeterminate: MdiMinus,
      checkboxOff: MdiCheckboxBlankOutline,
      checkboxOn: MdiCheckboxMarked,
      clear: MdiClose,
      close: MdiClose,
      collapse: MdiChevronUp,
      complete: MdiCheckCircle,
      delete: MdiCloseCircle,
      delimiter: MdiCircle,
      dropdown: MdiChevronDown,
      edit: MdiPencil,
      error: MdiAlertCircle,
      expand: MdiChevronDown,
      eye: MdiEye,
      eyeOff: MdiEyeOff,
      file: MdiPaperclip,
      first: MdiChevronDoubleLeft,
      info: MdiInformation,
      last: MdiChevronDoubleRight,
      loading: MdiRefresh,
      menu: MdiMenu,
      minus: MdiMinus,
      next: MdiChevronRight,
      plus: MdiPlus,
      prev: MdiChevronLeft,
      radioOff: MdiRadioboxBlank,
      radioOn: MdiRadioboxMarked,
      ratingEmpty: MdiStarOutline,
      ratingFull: MdiStar,
      ratingHalf: MdiStarHalfFull,
      sortAsc: MdiArrowUp,
      sortDesc: MdiArrowDown,
      subgroup: MdiChevronDown,
      success: MdiCheckCircle,
      treeviewCollapse: MdiChevronDown,
      treeviewExpand: MdiChevronRight,
      unfold: MdiChevronDown,
      warning: MdiAlert,
    },
  },
})
