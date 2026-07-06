import { THUMBNAIL_SIZES, VIDEO_SIZES, WEATHER_ICONS } from '@/scripts/constants.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import type { Location } from '@/scripts/types/api/fullPhoto.ts'
import { type RemovableRef, StorageSerializers, useStorage } from '@vueuse/core'

export function prettyBytes(bytes: number, decimals = 2): string {
  if (bytes === 0) return '0 B'

  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(decimals))} ${sizes[i]}`
}

export function requestIdleCallbackAsync(
  cb: (deadline: IdleDeadline) => Promise<void>,
): Promise<void> {
  return new Promise((resolve, reject) => {
    requestIdleCallback(async (deadline) => {
      try {
        await cb(deadline)
        resolve()
      } catch (err) {
        reject(err)
      }
    })
  })
}

export function toHms(totalSeconds: number) {
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = Math.round(totalSeconds % 60)
  if (hours > 0)
    return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`

  return `${minutes}:${seconds.toString().padStart(2, '0')}`
}

export function getThumbnailHeight(rowHeight: number) {
  for (const size of THUMBNAIL_SIZES) {
    if (size > rowHeight) return size
  }
  return THUMBNAIL_SIZES[THUMBNAIL_SIZES.length - 1]!
}

export function getVideoHeight(rowHeight: number) {
  for (const size of VIDEO_SIZES) {
    if (size > rowHeight) return size
  }
  return VIDEO_SIZES[VIDEO_SIZES.length - 1]!
}

export const stringToColor = (
  str: string,
  saturation: number = 50,
  lightness: number = 75,
): string => {
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    hash = str.charCodeAt(i) + ((hash << 5) - hash)
  }
  const hue = Math.abs(hash % 360)
  return `hsl(${hue}, ${saturation}%, ${lightness}%)`
}

export async function copyToClipboard(text: string) {
  const snackbarStore = useSnackbarsStore()
  try {
    await navigator.clipboard.writeText(text)
    snackbarStore.success('Copied to clipboard')
  } catch (e) {
    snackbarStore.error("Can't copy to clipboard", e)
  }
}

function base64UrlDecode(input: string): string {
  const base64 = input.replace(/-/g, '+').replace(/_/g, '/')

  const padded = base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), '=')

  return atob(padded)
}

export function isLikelyJwt(token: string): boolean {
  const parts = token.trim().split('.')

  if (parts.length !== 3) {
    return false
  }

  try {
    const header = JSON.parse(base64UrlDecode(parts[0]))
    const payload = JSON.parse(base64UrlDecode(parts[1]))

    if (typeof header !== 'object' || typeof payload !== 'object') {
      return false
    }

    if (!header.alg) {
      return false
    }

    return true
  } catch {
    return false
  }
}

export function getWeatherIcon(condition: string, isDaytime: boolean): string {
  const icon = WEATHER_ICONS[condition]

  if (!icon) {
    return 'cloudy_day_night.svg'
  }

  if (typeof icon === 'string') {
    return icon
  }

  return isDaytime ? icon.day : icon.night
}

export function makeDateTimeString(date: Date) {
  const day = date.getDate()
  const month = date.toLocaleString('en-GB', { month: 'long' })
  const year = date.getFullYear()

  const hours = String(date.getHours()).padStart(2, '0')
  const minutes = String(date.getMinutes()).padStart(2, '0')

  return `${day} ${month} ${year} at ${hours}:${minutes}`
}

export function makeLocationString(location: Location, components = 2) {
  let finalParts
  if (components === 3 && location.name && location.admin1 && location.country_name) {
    finalParts = [location.name, location.admin1, location.country_name]
  } else if (location.name && location.admin1) {
    finalParts = [location.name, location.admin1]
  } else {
    const prioritizedParts = [
      location.name,
      location.admin2,
      location.admin1,
      location.country_name,
    ]
    finalParts = prioritizedParts.filter((part) => part).slice(0, components)
  }
  finalParts = [...new Set(finalParts)]
  const result = finalParts.join(' - ')
  return result ? result : ''
}

export function formatNaiveDate(date: Date): string {
  const pad = (n: number): string => n.toString().padStart(2, '0')

  return (
    `${date.getFullYear()}-` +
    `${pad(date.getMonth() + 1)}-` +
    `${pad(date.getDate())}T` +
    `${pad(date.getHours())}:` +
    `${pad(date.getMinutes())}:` +
    `${pad(date.getSeconds())}`
  )
}

export function useObjStorage<T>(
  key: string,
  initialValue: T,
  storage: Storage = localStorage,
): RemovableRef<T> {
  return useStorage<T>(key, initialValue, storage, {
    serializer: StorageSerializers.object,
  })
}

export function caps(str: string) {
  if (str.length === 0) return str
  return str[0].toUpperCase() + str.slice(1)
}
