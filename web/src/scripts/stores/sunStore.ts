import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { SunCache } from '@/scripts/themeUtils.ts'
import { useObjStorage } from '@/scripts/utils.ts'

interface GeoResponse {
  latitude: string
  longitude: string
}

const CACHE_DURATION = 12 * 60 * 60 * 1000 // 12 hours in milliseconds

export const useSunStore = defineStore('sun', () => {
  const sunData = useObjStorage<SunCache | null>('sun_cache', null)
  const sunrise = computed(() => (sunData.value ? new Date(sunData.value.sunrise) : null))
  const sunset = computed(() => (sunData.value ? new Date(sunData.value.sunset) : null))

  const loading = ref<boolean>(false)
  const error = ref<string | null>(null)

  async function fetchSunTimes(useCache: boolean = true): Promise<void> {
    if (useCache && sunData.value) {
      const age = Date.now() - sunData.value.lastFetched
      if (age < CACHE_DURATION) {
        return
      }
    }

    loading.value = true
    error.value = null

    try {
      // 1. Get approximate location via IP
      const geoResponse = await fetch('https://get.geojs.io/v1/ip/geo.json')
      if (!geoResponse.ok) throw new Error('Failed to fetch location data')
      const geoData = (await geoResponse.json()) as GeoResponse
      const { latitude, longitude } = geoData

      // 2. Get sunrise and sunset times based on the coordinates
      const sunResponse = await fetch(
        `https://api.sunrise-sunset.org/json?lat=${latitude}&lng=${longitude}&formatted=0`,
      )
      if (!sunResponse.ok) throw new Error('Failed to fetch sun times')
      const sunDataJson = await sunResponse.json()

      const fetchedSunrise = new Date(sunDataJson.results.sunrise)
      const fetchedSunset = new Date(sunDataJson.results.sunset)
      sunData.value = {
        sunrise: fetchedSunrise.toISOString(),
        sunset: fetchedSunset.toISOString(),
        lastFetched: Date.now(),
      }
    } catch (err: unknown) {
      if (err instanceof Error) {
        error.value = err.message
      } else {
        error.value = 'An unexpected error occurred'
      }
      console.error(err)
    } finally {
      loading.value = false
    }
  }

  return { sunrise, sunset, loading, error, fetchSunTimes, sunData }
})
