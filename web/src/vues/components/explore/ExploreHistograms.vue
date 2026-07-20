<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'

const exploreStore = useExploreStore()

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

// Week mapping (1 to 53)
const weeksData = computed(() => {
  if (!exploreStore.histograms?.weekOfYear) return []

  const result = []
  for (let w = 1; w <= 53; w++) {
    const bucket = exploreStore.histograms?.weekOfYear.find((b) => b.week === w)
    result.push({
      week: w,
      count: bucket?.count || 0,
    })
  }
  return result
})

const maxWeekCount = computed(() => {
  const counts = weeksData.value.map((w) => w.count)
  return counts.length > 0 ? Math.max(...counts, 1) : 1
})

// Helper to determine approximate month label positions for 53 weeks
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
  return match ? match.label : null
}
</script>

<template>
  <div class="histograms-container">
    <div v-if="exploreStore.isHistogramsLoading" class="loading-state">
      <v-progress-circular indeterminate color="primary" size="64" />
      <p class="loading-text">Loading insights...</p>
    </div>

    <div v-else-if="exploreStore.histograms" class="histograms-grid">
      <!-- Top Row: Day of Week & Hour of Day -->
      <div class="top-row">
        <!-- Day of Week Card -->
        <v-card class="histogram-card" flat>
          <div class="card-header">
            <v-icon icon="mdi-calendar-week" class="card-icon" />
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
                v-tooltip="{
                  text: `${day.fullName}: ${day.count} photos & videos`,
                  location: 'top',
                }"
              >
                <div class="bar-container">
                  <div
                    class="bar-fill"
                    :style="{ height: `${(day.count / maxDayCount) * 100}%` }"
                  />
                </div>
                <span class="column-label">{{ day.label }}</span>
              </div>
            </div>
          </div>
        </v-card>

        <!-- Hour of Day Card -->
        <v-card class="histogram-card" flat>
          <div class="card-header">
            <v-icon icon="mdi-clock-outline" class="card-icon" />
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
                class="chart-column"
                v-tooltip="{
                  text: `${hour.label}: ${hour.count} photos & videos`,
                  location: 'top',
                }"
              >
                <div class="bar-container">
                  <div
                    class="bar-fill"
                    :style="{ height: `${(hour.count / maxHourCount) * 100}%` }"
                  />
                </div>
                <span v-if="hour.hour % 4 === 0" class="column-label">{{
                  hour.hour.toString().padStart(2, '0')
                }}</span>
                <span v-else class="column-label spacer" />
              </div>
            </div>
          </div>
        </v-card>
      </div>

      <!-- Bottom Row: Seasonal / Week of Year -->
      <v-card class="histogram-card full-width-card" flat>
        <div class="card-header">
          <v-icon icon="mdi-weather-partly-cloudy" class="card-icon" />
          <div class="header-texts">
            <h3 class="card-title">Seasonal Trends</h3>
            <p class="card-subtitle">Distribution of photos and videos over 53 weeks of the year</p>
          </div>
        </div>

        <div class="chart-wrapper">
          <div class="bar-chart week-chart">
            <div
              v-for="week in weeksData"
              :key="week.week"
              class="chart-column thin-column"
              v-tooltip="{
                text: `Week ${week.week}: ${week.count} photos & videos`,
                location: 'top',
              }"
            >
              <div class="bar-container">
                <div
                  class="bar-fill"
                  :style="{ height: `${(week.count / maxWeekCount) * 100}%` }"
                />
              </div>
              <span class="column-label week-label">
                {{ getMonthLabelForWeek(week.week) || '' }}
              </span>
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
  margin-bottom: 32px;
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

.top-row {
  display: grid;
  grid-template-columns: 1fr;
  gap: 28px;
}

@media (min-width: 960px) {
  .top-row {
    grid-template-columns: 1fr 1fr;
  }
}

.histogram-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  padding: 24px;
  border: none !important;
}

.card-header {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 24px;
}

.card-icon {
  color: rgb(var(--v-theme-primary));
  font-size: 28px;
}

.header-texts {
  display: flex;
  flex-direction: column;
}

.card-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.card-subtitle {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.chart-wrapper {
  padding-top: 8px;
}

.bar-chart {
  display: flex;
  align-items: flex-end;
  height: 180px;
  gap: 8px;
  position: relative;
}

.chart-column {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
  height: 100%;
  cursor: pointer;
  transition: opacity 0.15s ease;
}

.chart-column:hover {
  opacity: 0.85;
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
  transition: height 0.6s cubic-bezier(0.16, 1, 0.3, 1);
}

.column-label {
  margin-top: 8px;
  font-size: 0.75rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface-variant));
  height: 16px;
  text-align: center;
}

.column-label.spacer {
  visibility: hidden;
}

/* Day chart adjustments */
.day-chart {
  gap: 14px;
}

.day-chart .bar-container,
.day-chart .bar-fill {
  border-radius: 12px;
}

/* Hour chart adjustments */
.hour-chart {
  gap: 4px;
}

.hour-chart .bar-container,
.hour-chart .bar-fill {
  border-radius: 6px;
}

/* Week chart adjustments */
.week-chart {
  gap: 3px;
  height: 160px;
}

.thin-column {
  flex: 1;
}

.week-chart .bar-container,
.week-chart .bar-fill {
  border-radius: 3px;
}

.week-label {
  font-size: 0.7rem;
  white-space: nowrap;
  position: relative;
  /* Slightly shift to look beautifully balanced on thin columns */
  transform: translateX(10px);
}
</style>
