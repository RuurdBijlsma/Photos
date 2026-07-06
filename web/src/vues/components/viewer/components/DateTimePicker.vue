<script setup lang="ts">
import { computed, ref } from 'vue'

const props = defineProps<{
  modelValue: Date
  disabled?: boolean
}>()

const emit = defineEmits(['update:modelValue'])

const dateMenu = ref(false)
const timeMenu = ref(false)

/**
 * Merge new Date (YMD) into the current modelValue (preserving Time)
 */
function onDateInput(newDate: unknown) {
  if (!(newDate instanceof Date)) return
  const updated = new Date(props.modelValue)
  updated.setFullYear(newDate.getFullYear(), newDate.getMonth(), newDate.getDate())
  emit('update:modelValue', updated)
}

/**
 * Merge new Time (HMS) into the current modelValue (preserving Date)
 */
function onTimeInput(timeString: string | null) {
  if (!timeString) return
  const [hours, minutes, seconds] = timeString.split(':').map(Number)
  const updated = new Date(props.modelValue)
  updated.setHours(hours || 0, minutes || 0, seconds || 0)
  emit('update:modelValue', updated)
}

// Formatters for labels and pickers
const dateLabel = computed(() => {
  return props.modelValue.toLocaleDateString('en-GB', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  })
})

const timeLabel = computed(() => {
  return props.modelValue.toLocaleTimeString('en-GB', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
})
</script>

<template>
  <div class="datetime-input-wrapper" :class="{ 'is-disabled': disabled }">
    <!-- DATE BUTTON & MENU -->
    <v-menu
      v-model="dateMenu"
      :close-on-content-click="false"
      location="bottom center"
      :disabled="disabled"
    >
      <template v-slot:activator="{ props: menuProps }">
        <v-btn v-bind="menuProps" variant="text" density="compact" rounded="xl">
          {{ dateLabel }}
        </v-btn>
      </template>

      <v-card border rounded="lg">
        <v-date-picker
          bg-color="surface-container"
          color="primary"
          :model-value="modelValue"
          @update:model-value="onDateInput"
          hide-header
        />
        <v-divider />
        <v-card-actions class="justify-end">
          <v-btn variant="text" color="primary" @click="dateMenu = false">OK</v-btn>
        </v-card-actions>
      </v-card>
    </v-menu>

    <!-- TIME BUTTON & MENU -->
    <v-menu
      v-model="timeMenu"
      :close-on-content-click="false"
      location="bottom center"
      :disabled="disabled"
    >
      <template v-slot:activator="{ props: menuProps }">
        <v-btn v-bind="menuProps" variant="text" density="compact" rounded="xl">
          {{ timeLabel }}
        </v-btn>
      </template>

      <v-card border rounded="lg">
        <v-time-picker
          bg-color="surface-container"
          color="primary"
          :model-value="timeLabel"
          use-seconds
          @update:model-value="onTimeInput"
          format="24hr"
        />
        <v-divider />
        <v-card-actions class="justify-end">
          <v-btn variant="text" color="primary" @click="timeMenu = false">OK</v-btn>
        </v-card-actions>
      </v-card>
    </v-menu>

    <v-icon
      v-if="!disabled"
      icon="mdi-calendar-edit"
      size="small"
      color="primary"
      class="ml-2"
      style="opacity: 0.6"
    />
  </div>
</template>

<style scoped>
.datetime-input-wrapper {
  background-color: rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
  font-family: 'Roboto Mono', monospace;
  flex-grow: 1;
  padding: 10px 16px;
  border-radius: 10px;
  font-size: 16px;
  display: flex;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  align-items: center;
}

.datetime-input-wrapper.is-disabled {
  background-color: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.6);
  border-color: transparent;
  box-shadow: none;
  pointer-events: none;
}
</style>
