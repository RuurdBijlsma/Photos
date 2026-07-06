<script setup lang="ts">
import type { Weather } from '@/scripts/types/api/fullPhoto.js'
import { getWeatherIcon } from '@/scripts/utils.js'

defineProps<{
  weatherInfo: Weather
}>()
</script>

<template>
  <div class="weather-info">
    <img
      v-if="weatherInfo.condition && weatherInfo.is_daytime"
      :alt="`Icon for ${weatherInfo.condition} weather`"
      height="40"
      :src="`/img/weather/${getWeatherIcon(weatherInfo.condition, weatherInfo.is_daytime)}`"
    />
    <span v-if="weatherInfo.temperature">
      <span class="temperature">{{ weatherInfo.temperature }}</span
      ><span class="temp-unit"> °C</span>
    </span>
  </div>
</template>

<style scoped>
.weather-info {
  padding: 10px;
  align-items: center;
  display: flex;
  gap: 15px;
}

.temperature {
  font-size: 25px;
}

.temp-unit {
  font-size: 18px;
  opacity: 0.8;
}
</style>
