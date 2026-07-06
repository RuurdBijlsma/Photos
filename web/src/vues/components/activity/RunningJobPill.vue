<script setup lang="ts">
import { caps } from '@/scripts/utils.ts'

defineProps<{
  jobType: string
  relativePath?: string | null
  compact?: boolean
}>()
</script>

<template>
  <div :class="['job-pill', compact ? 'pa-3' : 'pa-4']">
    <div class="job-info-left">
      <span class="job-type-badge" :class="{ 'text-caption mb-1': !compact }">
        {{ caps(jobType.replace('ingest_', '')) }}
      </span>
      <span
        class="job-path"
        :class="{ 'font-weight-medium': !compact }"
        :title="relativePath || ''"
      >
        {{
          relativePath ? (compact ? relativePath.split('/').pop() : relativePath) : 'System Task'
        }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.job-pill {
  background-color: rgb(var(--v-theme-surface-container-low));
  border-radius: 18px;
  margin-bottom: 8px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.job-info-left {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.job-type-badge {
  font-size: 0.725rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: rgb(var(--v-theme-primary));
}

.job-path {
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface));
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}
</style>
