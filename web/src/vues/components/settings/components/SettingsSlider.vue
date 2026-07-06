<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  modelValue: number
  label: string
  description?: string
  min?: number
  max?: number
  step?: number
  resetValue?: number
  unit?: string
  showTicks?: boolean
  formatValue?: (val: number) => string
}

const props = withDefaults(defineProps<Props>(), {
  min: 0,
  max: 100,
  step: 1,
  unit: '',
  showTicks: false,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
  (e: 'slide-start'): void
}>()

const formattedValue = computed(() => {
  if (props.formatValue) {
    return props.formatValue(props.modelValue)
  }
  return props.modelValue.toString()
})

const reset = () => {
  if (props.resetValue !== undefined) {
    emit('update:modelValue', props.resetValue)
  }
}
</script>

<template>
  <div class="slider-wrapper">
    <span class="slider-label">
      {{ label }}
      <span class="contrast-value-label">{{ formattedValue }}</span
      >{{ unit }}
    </span>
    <div class="slider-flex">
      <v-slider
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
        @mousedown="emit('slide-start')"
        @touchstart="emit('slide-start')"
        :min="min"
        :max="max"
        :step="step"
        :show-ticks="showTicks"
        color="primary"
        hide-details
        class="mt-2"
      />
      <v-btn
        v-if="resetValue !== undefined"
        class="slider-reset-button"
        color="tertiary"
        @click="reset"
        rounded
        density="compact"
        variant="plain"
        >Reset</v-btn
      >
    </div>
    <slot name="description">
      <span v-if="description" class="slider-desc" v-html="description" />
    </slot>
  </div>
</template>

<style scoped>
.slider-wrapper {
  background-color: rgb(var(--v-theme-surface-container-high));
  padding: 16px;
  border-radius: 12px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.slider-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
  display: block;
}

.contrast-value-label {
  font-weight: 700;
  min-width: 44px;
  margin-left: 10px;
  text-align: right;
  color: rgb(var(--v-theme-primary));
}

.slider-flex {
  display: flex;
  align-items: center;
  gap: 10px;
}

.slider-reset-button {
  transform: translateY(3px);
}

.slider-desc {
  font-size: 0.75rem;
  color: rgb(var(--v-theme-on-surface-variant));
  display: block;
}
</style>
