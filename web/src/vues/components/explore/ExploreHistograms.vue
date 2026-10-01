<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'
import MdiCalendarWeekOutline from '~icons/mdi/calendar-week-outline'
import MdiClockOutline from '~icons/mdi/clock-outline'
import MdiCalendarMonthOutline from '~icons/mdi/calendar-month-outline'

const exploreStore = useExploreStore()
const { isMobile } = useResponsive()

onMounted(async () => {
  if (!exploreStore.histograms) {
    await exploreStore.fetchHistograms()
  }
})

// Day mapping order (Monday = 1, Tuesday = 2, ..., Saturday = 6, Sunday = 0)
const DAYS_ORDER = [1, 2, 3, 4, 5, 6, 0]
const DAY_LABELS_SHORT = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

// Full day list sorted Mon-Sun
const daysData = computed(() => {
  if (!exploreStore.histograms?.dayOfWeek) return []

  return DAYS_ORDER.map((dayNum, idx) => {
    const bucket = exploreStore.histograms?.dayOfWeek.find((b) => b.day === dayNum)
    return {
      label: DAY_LABELS_SHORT[idx],
      fullName: bucket?.label || 'Unknown',
      count: bucket?.count || 0,
    }
  })
})

const maxDayCount = computed(() => {
  const counts = daysData.value.map((d) => d.count)
  return counts.length > 0 ? Math.max(...counts, 1) : 1
})

// Hour mapping (0 to 23)
const hoursData = computed(() => {
  if (!exploreStore.histograms?.hourOfDay) return []

  const result = []
  for (let h = 0; h < 24; h++) {
    const bucket = exploreStore.histograms?.hourOfDay.find((b) => b.hour === h)
    const nextH = (h + 1) % 24
    result.push({
      hour: h,
      label: `${h}:00 - ${nextH}:00`,
      count: bucket?.count || 0,
    })
  }
  return result
})

const maxHourCount = computed(() => {
  const counts = hoursData.value.map((h) => h.count)
  return counts.length > 0 ? Math.max(...counts, 1) : 1
})

// Week mapping (1 to 52)
const weeksData = computed(() => {
  if (!exploreStore.histograms?.weekOfYear) return []

  const result = []
  for (let w = 1; w <= 52; w++) {
    const bucket = exploreStore.histograms?.weekOfYear.find((b) => b.week === w)
    let count = bucket?.count || 0

    // Merge week 53 into week 52 if it exists
    if (w === 52) {
      const week53Bucket = exploreStore.histograms?.weekOfYear.find((b) => b.week === 53)
      if (week53Bucket) {
        count += week53Bucket.count
      }
    }

    result.push({
      week: w,
      count,
    })
  }
  return result
})

const maxWeekCount = computed(() => {
  const counts = weeksData.value.map((w) => w.count)
  return counts.length > 0 ? Math.max(...counts, 1) : 1
})

// Helper to determine approximate month label positions for 52 weeks
const monthLabels = [
  { week: 1, label: 'Jan' },
  { week: 5, label: 'Feb' },
  { week: 9, label: 'Mar' },
  { week: 13, label: 'Apr' },
  { week: 18, label: 'May' },
  { week: 22, label: 'Jun' },
  { week: 27, label: 'Jul' },
  { week: 31, label: 'Aug' },
  { week: 36, label: 'Sep' },
  { week: 40, label: 'Oct' },
  { week: 44, label: 'Nov' },
  { week: 49, label: 'Dec' },
]

function getMonthLabelForWeek(weekNum: number): string | null {
  const match = monthLabels.find((m) => m.week === weekNum)
  if (!match) return null

  // On mobile, show alternate months to avoid text collisions
  if (isMobile.value) {
    const mobileVisibleWeeks = [1, 9, 18, 27, 36, 44]
    return mobileVisibleWeeks.includes(weekNum) ? match.label : null
  }
  return match.label
}
</script>

<template>
  <div class="histograms-container">
    <!-- Histograms Grid -->
    <div class="histograms-grid">
      <!-- Top Row: Day of Week & Time of Day -->
      <div class="top-row">
        <!-- Day of Week Card -->
        <v-card class="histogram-card" flat elevation="0">
          <div class="card-header">
            <v-icon class="card-icon" :icon="MdiCalendarWeekOutline" />
            <div class="header-texts">
              <h3 class="card-title">Weekly Habits</h3>
              <p class="card-subtitle">Media volume captured across days of the week</p>
            </div>
          </div>

          <div class="chart-wrapper">
            <div class="bar-chart day-chart">
              <div
                v-for="day in daysData"
                :key="day.label"
                class="chart-column"
                :title="`${day.fullName}: ${day.count} photos & videos`"
              >
                <div class="bar-container">
                  <div
                    class="bar-fill"
                    :style="{ height: `${maxDayCount > 0 ? (day.count / maxDayCount) * 100 : 0}%` }"
                  />
                </div>
                <span class="column-label">{{ day.label }}</span>
              </div>
            </div>
          </div>
        </v-card>

        <!-- Time of Day Card -->
        <v-card class="histogram-card" flat elevation="0">
          <div class="card-header">
            <v-icon class="card-icon" :icon="MdiClockOutline" />
            <div class="header-texts">
              <h3 class="card-title">Daily Rhythm</h3>
              <p class="card-subtitle">Activity trends mapped by hour of the day</p>
            </div>
          </div>

          <div class="chart-wrapper">
            <div class="bar-chart hour-chart">
              <div
                v-for="hour in hoursData"
                :key="hour.hour"
                class="chart-column thin-column"
                :title="`${hour.label}: ${hour.count} photos & videos`"
              >
                <div class="bar-container">
                  <div
                    class="bar-fill"
                    :style="{
                      height: `${maxHourCount > 0 ? (hour.count / maxHourCount) * 100 : 0}%`,
                    }"
                  />
                </div>
                <span
                  class="column-label"
                  :class="{ spacer: hour.hour % (isMobile ? 6 : 3) !== 0 }"
                >
                  {{ hour.hour % (isMobile ? 6 : 3) === 0 ? `${hour.hour}h` : '' }}
                </span>
              </div>
            </div>
          </div>
        </v-card>
      </div>

      <!-- Bottom Card: Seasonality (Week of Year) -->
      <v-card class="histogram-card" flat elevation="0">
        <div class="card-header">
          <v-icon class="card-icon" :icon="MdiCalendarMonthOutline" />
          <div class="header-texts">
            <h3 class="card-title">Seasonal Trends</h3>
            <p class="card-subtitle">Distribution of photos and videos over 52 weeks of the year</p>
          </div>
        </div>

        <div class="chart-wrapper">
          <div class="bar-chart week-chart">
            <div
              v-for="week in weeksData"
              :key="week.week"
              class="chart-column thin-column"
              :title="`Week ${week.week}: ${week.count} photos & videos`"
            >
              <div class="bar-container">
                <div
                  class="bar-fill"
                  :style="{
                    height: `${maxWeekCount > 0 ? (week.count / maxWeekCount) * 100 : 0}%`,
                  }"
                />
              </div>
              <span v-if="getMonthLabelForWeek(week.week)" class="column-label week-label">
                {{ getMonthLabelForWeek(week.week) }}
              </span>
              <span v-else class="column-label spacer" />
            </div>
          </div>
        </div>
      </v-card>
    </div>
  </div>
</template>

<style scoped>
.histograms-container {
  width: 100%;
  margin-bottom: 28px;
}

.is-mobile.histograms-container {
  margin-bottom: 20px;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px;
  background-color: rgb(var(--v-theme-surface-container-low));
  border-radius: 28px;
}

.loading-text {
  margin-top: 16px;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.95rem;
}

.histograms-grid {
  display: flex;
  flex-direction: column;
  gap: 28px;
}

.is-mobile .histograms-grid {
  gap: 16px;
}

.top-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 28px;
}

.is-mobile .top-row {
  grid-template-columns: 1fr;
  gap: 16px;
}

.histogram-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  padding: 24px;
  border: none !important;
  overflow: hidden;
}

.is-mobile .histogram-card {
  border-radius: 22px !important;
  padding: 18px 14px;
}

.card-header {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 24px;
}

.is-mobile .card-header {
  gap: 12px;
  margin-bottom: 16px;
}

.card-icon {
  color: rgb(var(--v-theme-primary));
  font-size: 28px;
  margin-top: 2px;
  flex-shrink: 0;
}

.is-mobile .card-icon {
  font-size: 22px;
}

.header-texts {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.card-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.is-mobile .card-title {
  font-size: 1.1rem;
}

.card-subtitle {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.is-mobile .card-subtitle {
  font-size: 0.78rem;
}

.chart-wrapper {
  padding-top: 8px;
  width: 100%;
}

.bar-chart {
  display: flex;
  align-items: flex-end;
  height: 180px;
  gap: 8px;
  position: relative;
  width: 100%;
  touch-action: pan-y;
}

.is-mobile .bar-chart {
  height: 140px;
}

.chart-column {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  height: 100%;
  transition: opacity 0.15s ease;
  user-select: none;
  -webkit-user-select: none;
}

.chart-column:hover {
  opacity: 0.85;
}

.chart-column:active {
  opacity: 0.7;
}

.bar-container {
  flex-grow: 1;
  width: 100%;
  position: relative;
  background-color: rgba(var(--v-theme-on-surface), 0.05);
  border-radius: 8px;
  overflow: hidden;
}

.bar-fill {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: linear-gradient(
    180deg,
    rgba(var(--v-theme-primary), 0.8) 0%,
    rgba(var(--v-theme-primary), 0.7) 100%
  );
  border-radius: 8px;
  transition: height 0.5s cubic-bezier(0.16, 1, 0.3, 1);
}

.column-label {
  margin-top: 8px;
  font-size: 0.75rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface-variant));
  height: 16px;
  text-align: center;
  white-space: nowrap;
}

.is-mobile .column-label {
  font-size: 0.68rem;
  margin-top: 6px;
}

.column-label.spacer {
  visibility: hidden;
}

/* Day chart adjustments */
.day-chart {
  gap: 14px;
}

.is-mobile .day-chart {
  gap: 6px;
}

.day-chart .bar-container,
.day-chart .bar-fill {
  border-radius: 12px;
}

.is-mobile .day-chart .bar-container,
.is-mobile .day-chart .bar-fill {
  border-radius: 8px;
}

/* Hour chart adjustments */
.hour-chart {
  gap: 4px;
}

.is-mobile .hour-chart {
  gap: 1.5px;
}

.hour-chart .bar-container,
.hour-chart .bar-fill {
  border-radius: 6px;
}

.is-mobile .hour-chart .bar-container,
.is-mobile .hour-chart .bar-fill {
  border-radius: 3px;
}

.thin-column {
  flex: 1;
  min-width: 0;
  position: relative;
}

/* Week chart adjustments */
.week-chart {
  gap: 2px;
  height: 160px;
}

.is-mobile .week-chart {
  gap: 1px;
  height: 130px;
}

.week-chart .bar-container,
.week-chart .bar-fill {
  border-radius: 4px;
}

.is-mobile .week-chart .bar-container,
.is-mobile .week-chart .bar-fill {
  border-radius: 2px;
}

/* Month labels across 52 weeks */
.week-label {
  font-size: 0.7rem;
  white-space: nowrap;
  width: 0;
  overflow: visible;
  display: flex;
  justify-content: center;
  position: relative;
  transform: translateX(8px);
}

.is-mobile .week-label {
  font-size: 0.65rem;
  transform: translateX(4px);
}
</style>
