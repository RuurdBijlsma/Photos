<script setup lang="ts">
import { computed } from 'vue'
import { formatEta } from '@/scripts/utils.ts'

const props = withDefaults(
  defineProps<{
    label: string
    icon: string
    progress: number
    status: string
    isActive?: boolean
    toGo?: number
    itemsPerSecond?: number | null
    tooltipText?: string
  }>(),
  {
    isActive: false,
    toGo: 0,
    itemsPerSecond: 0,
    tooltipText: '',
  },
)

const showMetrics = computed(() => {
  return props.toGo > 50 && props.itemsPerSecond !== null && props.itemsPerSecond > 0
})

const formattedEta = computed(() => {
  if (!showMetrics.value || !props.itemsPerSecond || props.itemsPerSecond <= 0) return ''
  const seconds = props.toGo / props.itemsPerSecond
  return formatEta(seconds)
})

const formattedSpeed = computed(() => {
  if (!props.itemsPerSecond || props.itemsPerSecond <= 0) return ''
  if (props.itemsPerSecond >= 10) {
    return `${Math.round(props.itemsPerSecond)} items/s`
  }
  return `${props.itemsPerSecond.toFixed(1)} items/s`
})
</script>

<template>
  <div
    class="pipeline-step-wrapper"
    v-tooltip="{
      location: 'top',
      text: tooltipText,
      disabled: !tooltipText,
    }"
  >
    <div class="pipeline-step-card" :class="{ active: isActive }">
      <v-progress-circular
        :model-value="progress"
        color="primary"
        size="88"
        width="6"
        class="pipeline-circle"
      >
        <div class="circle-inner">
          <v-icon size="default">{{ icon }}</v-icon>
          <span class="circle-pct">{{ Math.round(progress) }}%</span>
        </div>
      </v-progress-circular>
      <div class="step-details">
        <span class="step-label">{{ label }}</span>
        <span class="step-status">{{ status }}</span>
        <template v-if="showMetrics">
          <span class="step-speed">{{ formattedSpeed }}</span>
          <span class="step-eta" v-if="formattedEta">{{ formattedEta }} left</span>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pipeline-step-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.pipeline-step-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 24px;
  border-radius: 24px;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  background-color: rgb(var(--v-theme-surface-container-low));
  min-width: 140px;
  min-height: 206px;
}

@keyframes pulse-glow {
  0% {
    box-shadow: 0 0 0 0 rgba(var(--v-theme-primary), 0.2);
  }
  70% {
    box-shadow: 0 0 0 10px rgba(var(--v-theme-primary), 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(var(--v-theme-primary), 0);
  }
}

.pipeline-step-card.active {
  border: 1.5px solid rgba(var(--v-theme-primary), 0.3);
  animation: pulse-glow 2s infinite;
}

.pipeline-circle {
  background-color: rgb(var(--v-theme-surface-container-low));
  border-radius: 50%;
}

.circle-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.circle-pct {
  font-size: 0.75rem;
  font-weight: 700;
  margin-top: 2px;
}

.step-details {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  margin-top: 8px;
  justify-content: space-evenly;
  flex-grow: 1;
}

.step-label {
  font-size: 0.9rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.step-status {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.step-speed {
  font-size: 0.725rem;
  font-weight: 600;
  color: rgb(var(--v-theme-primary));
  margin-top: 2px;
}

.step-eta {
  font-size: 0.7rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.7);
}
</style>
