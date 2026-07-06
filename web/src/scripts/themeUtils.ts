export interface SunCache {
  sunrise: string
  sunset: string
  lastFetched: number
}

/**
 * Shared utility to resolve theme configurations into 'light', 'dark', or 'system' modes.
 */
export function resolveThemeMode(
  themeString: string,
  useSunSchedule: boolean,
  sunrise: Date | string | null,
  sunset: Date | string | null,
  lightTime: string,
  darkTime: string,
): 'light' | 'dark' | 'system' {
  if (themeString === 'light') return 'light'
  if (themeString === 'dark') return 'dark'
  if (themeString === 'system') return 'system'

  if (themeString === 'schedule') {
    const now = new Date()
    if (useSunSchedule) {
      if (sunrise && sunset) {
        const riseDate = new Date(sunrise)
        const setDate = new Date(sunset)
        const nowMin = now.getHours() * 60 + now.getMinutes()
        const riseMin = riseDate.getHours() * 60 + riseDate.getMinutes()
        const setMin = setDate.getHours() * 60 + setDate.getMinutes()

        if (riseMin < setMin) {
          return nowMin < riseMin || nowMin >= setMin ? 'dark' : 'light'
        } else {
          return nowMin >= setMin && nowMin < riseMin ? 'dark' : 'light'
        }
      } else {
        const nowHour = now.getHours()
        return nowHour < 6 || nowHour >= 18 ? 'dark' : 'light'
      }
    } else {
      const [lightH, lightM] = (lightTime || '07:00').split(':').map(Number)
      const [darkH, darkM] = (darkTime || '19:00').split(':').map(Number)

      const lightMin = lightH * 60 + lightM
      const darkMin = darkH * 60 + darkM
      const nowMin = now.getHours() * 60 + now.getMinutes()

      if (lightMin < darkMin) {
        return nowMin < lightMin || nowMin >= darkMin ? 'dark' : 'light'
      } else {
        return nowMin >= darkMin && nowMin < lightMin ? 'dark' : 'light'
      }
    }
  }

  return 'light'
}
