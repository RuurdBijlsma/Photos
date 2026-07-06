<script setup lang="ts">
import { ref, computed } from 'vue'
import HistogramDateRangePicker from '@/vues/components/ui/HistogramDateRangePicker.vue'

interface DateRange {
  startDate: Date | null
  endDate: Date | null
  active: boolean
  startGranularity: 'month' | 'day'
  endGranularity: 'month' | 'day'
}

const props = defineProps<{
  modelValue: DateRange
  theme: 'dark' | 'light'
}>()

const emit = defineEmits(['update:modelValue', 'change'])

const isOpen = ref(false)

// Format date for display on buttons
function formatDateLabel(date: Date | null, granularity: 'month' | 'day'): string {
  if (!date) return 'Select Date'
  if (granularity === 'month') {
    return date.toLocaleDateString('en-GB', { month: 'short', year: 'numeric' })
  }
  return date.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })
}

// The label displayed on the closed floating button
const closedButtonLabel = computed(() => {
  if (!props.modelValue.active || !props.modelValue.startDate || !props.modelValue.endDate) {
    return 'Date filter'
  }
  const startStr = formatDateLabel(props.modelValue.startDate, props.modelValue.startGranularity)
  const endStr = formatDateLabel(props.modelValue.endDate, props.modelValue.endGranularity)
  return `Date filtered from ${startStr} to ${endStr}`
})
</script>

<template>
  <v-theme-provider :theme="props.theme">
    <div class="map-date-filter">
      <!-- CLOSED STATE BUTTON -->
      <v-btn
        v-if="!isOpen"
        class="map-date-filter-chip"
        variant="flat"
        rounded="xl"
        elevation="4"
        prepend-icon="mdi-calendar-filter"
        @click="isOpen = true"
      >
        {{ closedButtonLabel }}
      </v-btn>

      <!-- OPEN STATE DIALOG CARD -->
      <v-card v-else class="map-date-filter-panel" elevation="8" rounded="xl">
        <!-- Close Button -->
        <v-btn
          icon="mdi-close"
          variant="text"
          density="comfortable"
          class="panel-close-btn"
          size="small"
          @click="isOpen = false"
        />

        <!-- Shared Histogram date range picker component -->
        <HistogramDateRangePicker
          :model-value="props.modelValue"
          @update:model-value="(val) => emit('update:modelValue', val)"
          @change="(payload) => emit('change', payload)"
        />
      </v-card>
    </div>
  </v-theme-provider>
</template>

<style>
.map-date-filter {
  position: absolute;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 20;
  font-family: inherit;
  display: flex;
  justify-content: center;
  pointer-events: auto; /* Ensure clicks reach the component */
}

/* Glassmorphism card for open panel */
.map-date-filter-panel {
  width: 520px;
  max-width: 90vw;
  background-color: rgba(var(--v-theme-surface), 0.72) !important;
  backdrop-filter: blur(16px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(16px) saturate(180%) !important;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12) !important;
  padding: 16px;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.35) !important;
  display: flex;
  flex-direction: column;
  position: relative;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

/* Semi-transparent closed button */
.map-date-filter-chip {
  background-color: rgba(var(--v-theme-surface), 0.85) !important;
  backdrop-filter: blur(12px) !important;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1) !important;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.2) !important;
  text-transform: none !important;
  font-weight: 500 !important;
  font-size: 14px !important;
  color: rgb(var(--v-theme-on-surface)) !important;
  height: 40px !important;
  padding: 0 20px !important;
}

.panel-close-btn {
  position: absolute !important;
  top: 8px;
  right: 8px;
  color: rgba(var(--v-theme-on-surface), 0.6);
  z-index: 10;
}
</style>
