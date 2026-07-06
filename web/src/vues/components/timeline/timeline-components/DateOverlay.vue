<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { CURRENT_YEAR, DAYS, MONTHS } from '@/scripts/constants.ts'

const props = defineProps<{
  date: Date | null
}>()
const emit = defineEmits(['datePicked'])

const renderedDate = computed(() => {
  const date = props.date
  if (date === null) return { date: null, year: null }

  const day = DAYS[date.getDay()]!
  const month = MONTHS[date.getMonth()]!
  const year = date.getFullYear() === CURRENT_YEAR ? null : ' ' + date.getFullYear()

  return { date: `${day.substring(0, 3)}, ${date.getDate()} ${month.substring(0, 3)}`, year }
})

const dateMenuOpen = ref(false)
const pickedDate = ref<Date | null>(null)

watch(
  () => props.date,
  (newDate) => {
    if (!dateMenuOpen.value) pickedDate.value = newDate
  },
  { immediate: true },
)

// Only close the menu and emit the update when the user actively selects a date in the open menu
watch(pickedDate, (newVal) => {
  if (dateMenuOpen.value) {
    dateMenuOpen.value = false
    emit('datePicked', newVal)
  }
})
</script>

<template>
  <v-menu
    :close-on-content-click="false"
    v-model="dateMenuOpen"
    position="bottom-left"
    location="bottom end"
  >
    <template v-slot:activator="{ props }">
      <v-slide-y-transition>
        <div class="date-view" v-if="date" v-bind="props">
          <span class="date-view-date">{{ renderedDate.date }}</span>
          <span v-if="renderedDate.year" class="date-view-year">{{ renderedDate.year }}</span>
        </div>
      </v-slide-y-transition>
    </template>
    <v-date-picker
      hide-header
      elevation="5"
      class="date-picker"
      control-variant="docked"
      rounded="xl"
      v-model="pickedDate"
    ></v-date-picker>
  </v-menu>
</template>

<style scoped>
.date-picker {
  background-color: rgba(var(--v-theme-surface-container), 0.95);
  border: 1px solid rgba(255, 255, 255, 0.1);
  margin-top: 5px;
}

.date-view {
  position: absolute;
  top: 30px;
  right: 30px;
  padding: 20px 40px;
  z-index: 3;
  text-align: left;
  font-weight: 500;
  border-radius: 40px;
  background-color: rgba(var(--v-theme-surface-container-high), 1);
  color: rgba(var(--v-theme-on-surface-container-high), 1);
  box-shadow:
    0 4px 8px 0 rgba(0, 0, 0, 0.2),
    0 6px 20px 0 rgba(0, 0, 0, 0.19);
}

.date-view-date {
  font-weight: 700;
  font-size: 22px;
  margin-right: 20px;
}

.date-view-year {
  font-weight: 400;
  font-size: 16px;
}
</style>
