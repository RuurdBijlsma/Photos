import { THUMBNAIL_SIZES, VIDEO_SIZES, WEATHER_ICONS } from '@/scripts/constants.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import type { Location } from '@/scripts/types/api/fullPhoto.ts'
import { type RemovableRef, StorageSerializers, useStorage } from '@vueuse/core'
import type { AxiosResponseHeaders, RawAxiosResponseHeaders } from 'axios'

export function filenameFromHeaders(headers?: RawAxiosResponseHeaders | AxiosResponseHeaders) {
  const contentDisposition = headers?.['content-disposition'] || headers?.['Content-Disposition']
  if (contentDisposition) {
    const match = contentDisposition.match(/filename\*?=(?:UTF-8'')?['"]?([^;\r\n"']+)['"]?/i)
    if (match && match[1]) {
      return decodeURIComponent(match[1])
    }
  }
}

export function downloadBlob(blob: Blob, filename?: string) {
  const url = window.URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  if (filename) link.download = filename
  link.click()
  setTimeout(() => {
    window.URL.revokeObjectURL(url)
  }, 60000)
}

export function prettyBytes(bytes: number): string {
  if (bytes === 0) return '0 B'

  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))

  const value = bytes / Math.pow(k, i)
  const integerDigits = Math.floor(value).toString().length
  const decimals = integerDigits >= 3 ? 0 : 1

  return `${parseFloat(value.toFixed(decimals))} ${sizes[i]}`
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

export function isMobileDevice(): boolean {
  if (typeof navigator === 'undefined') {
    return false
  }
  return /Android|iPhone|iPad|iPod|Mobile/i.test(navigator.userAgent)
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
  let iconName = 'cloudy_day_night.svg'

  if (icon) {
    iconName = typeof icon === 'string' ? icon : isDaytime ? icon.day : icon.night
  }

  return new URL(`../assets/img/weather/${iconName}`, import.meta.url).href
}

export function makeTimeString(date: Date) {
  const hours = String(date.getHours())
  const minutes = String(date.getMinutes()).padStart(2, '0')
  return `${hours}:${minutes}`
}

export function makeDateTimeString(date: Date) {
  const day = date.getDate()
  const month = date.toLocaleString('en-GB', { month: 'long' })
  const year = date.getFullYear()

  return `${day} ${month} ${year} at ${makeTimeString(date)}`
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

export function arrayToMap<T extends { id: string }>(items: readonly T[]): Map<string, T> {
  const map = new Map<string, T>()

  for (let i = 0; i < items.length; i++) {
    const item = items[i]
    map.set(item.id, item)
  }

  return map
}

export function formatEta(seconds: number): string {
  if (!isFinite(seconds) || seconds <= 0) return ''
  const hrs = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)

  if (hrs > 0) return `${hrs}h ${mins}m`
  if (mins > 0) return `${mins}m ${secs}s`
  return `${secs}s`
}

export class ProcessingRateTracker {
  private history: { timestamp: number; completed: number }[] = []
  private windowMs: number

  constructor(windowMs = 20000) {
    this.windowMs = windowMs
  }

  update(completed: number): number {
    const now = Date.now()

    if (this.history.length > 0 && completed < this.history[this.history.length - 1].completed) {
      this.history = []
    }

    this.history.push({ timestamp: now, completed })

    const cutoff = now - this.windowMs
    this.history = this.history.filter((h) => h.timestamp >= cutoff)

    if (this.history.length < 2) return 0

    const oldest = this.history[0]
    const newest = this.history[this.history.length - 1]
    const deltaCompleted = newest.completed - oldest.completed
    const deltaSec = (newest.timestamp - oldest.timestamp) / 1000

    if (deltaSec <= 0 || deltaCompleted <= 0) return 0
    return deltaCompleted / deltaSec
  }

  reset(): void {
    this.history = []
  }
}
