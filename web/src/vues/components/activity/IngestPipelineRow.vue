<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    label: string
    icon: string
    total: number
    percentage: number
    toGo: number
    counts?: {
      done: number
      running: number
      queued: number
      failed: number
      cancelled: number
    }
    compact?: boolean
  }>(),
  {
    compact: false,
  },
)

const segments = computed(() => {
  if (!props.counts || props.total === 0) return []
  return [
    { name: 'done', value: props.counts.done, colorClass: 'done', label: 'Completed' },
    { name: 'running', value: props.counts.running, colorClass: 'running', label: 'Running' },
    { name: 'queued', value: props.counts.queued, colorClass: 'queued', label: 'Queued' },
    { name: 'failed', value: props.counts.failed, colorClass: 'failed', label: 'Failed' },
    {
      name: 'cancelled',
      value: props.counts.cancelled,
      colorClass: 'cancelled',
      label: 'Cancelled',
    },
  ]
    .filter((s) => s.value > 0)
    .map((s) => ({
      ...s,
      pct: (s.value / props.total) * 100,
    }))
})
</script>

<template>
  <div :class="['category-row', compact ? 'compact-pad' : 'pa-5']">
    <div class="category-header">
      <span class="category-title">
        <v-icon :icon="icon" :size="compact ? 18 : 22" color="primary" />
        {{ label }}
      </span>
      <span class="category-stats" v-if="total > 0 && toGo > 0">
        <strong>{{ toGo.toLocaleString() }}</strong> of {{ total.toLocaleString() }} to go ({{
          percentage
        }}%)
      </span>
      <span class="category-stats" v-else-if="total > 0">
        <em>All done</em>
      </span>
      <span class="category-stats italic" v-else>
        {{ compact ? 'No jobs scheduled' : 'No active/recent jobs found' }}
      </span>
    </div>

    <!-- Custom stacked multi-segment progress bar -->
    <div class="progress-track" :class="{ 'height-large': !compact }" v-if="total > 0">
      <div
        v-for="seg in segments"
        :key="seg.name"
        :class="['progress-seg', seg.colorClass]"
        :style="{ width: seg.pct + '%' }"
        v-tooltip:top="`${seg.label}: ${seg.value}`"
      />
    </div>

    <!-- Custom Multi-row Legend Rendering -->
    <template v-if="total > 0">
      <div class="legend-flex">
        <div v-for="seg in segments" :key="seg.name" class="legend-item">
          <div :class="['legend-dot', seg.colorClass]" />
          <span
            >{{ seg.label }} (<strong>{{ seg.value.toLocaleString() }}</strong
            >)</span
          >
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.category-row {
  border-radius: 20px;
  display: flex;
  flex-direction: column;
  transition: all 0.3s ease;
  background-color: rgba(var(--v-theme-surface-container-low), 1);
}

.category-row.compact-pad {
  padding: 12px;
}

.category-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.category-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  font-size: 0.875rem;
}

.category-stats {
  font-size: 0.8rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.progress-track {
  display: flex;
  height: 6px;
  background-color: rgb(var(--v-theme-surface-container-highest));
  border-radius: 3px;
  overflow: hidden;
  margin-top: 8px;
  margin-bottom: 4px;
}

.progress-track.height-large {
  height: 10px;
  border-radius: 5px;
  margin-top: 12px;
  margin-bottom: 12px;
}

.progress-seg {
  height: 100%;
  transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

.progress-seg.done {
  background-color: rgb(var(--v-theme-success));
}
.progress-seg.running {
  background-color: rgb(var(--v-theme-info));
}
.progress-seg.queued {
  background-color: rgb(var(--v-theme-warning));
}
.progress-seg.failed {
  background-color: rgb(var(--v-theme-error));
}
.progress-seg.cancelled {
  background-color: rgba(var(--v-theme-on-surface), 0.35);
}

.legend-flex {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 6px;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.7rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.legend-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.legend-dot.done {
  background-color: rgb(var(--v-theme-success));
}
.legend-dot.running {
  background-color: rgb(var(--v-theme-info));
}
.legend-dot.queued {
  background-color: rgb(var(--v-theme-warning));
}
.legend-dot.failed {
  background-color: rgb(var(--v-theme-error));
}
.legend-dot.cancelled {
  background-color: rgba(var(--v-theme-on-surface), 0.35);
}

.italic {
  font-style: italic;
}
</style>
