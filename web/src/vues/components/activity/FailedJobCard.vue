<script setup lang="ts">
import { ref } from 'vue'
import { caps } from '@/scripts/utils.ts'

defineProps<{
  id: number
  jobType: string
  relativePath?: string | null
  attempts: number
  maxAttempts: number
  lastError?: string | null
  retrying: boolean
}>()

const emit = defineEmits<{
  (e: 'retry', jobId: number): void
}>()

const expanded = ref(false)
</script>

<template>
  <div class="failed-pill">
    <div class="failed-main pa-4">
      <div class="job-info-left">
        <span class="job-type-badge">
          {{ caps(jobType.replace('ingest_', '')) }}
        </span>
        <span class="job-path" :title="relativePath || ''">
          {{ relativePath ? relativePath.split('/').pop() : 'System Task' }}
        </span>
        <span class="attempts"> Attempts: {{ attempts }} / {{ maxAttempts }} </span>
      </div>

      <div class="failed-actions">
        <v-btn
          v-if="lastError"
          :icon="expanded ? 'mdi-chevron-up' : 'mdi-chevron-down'"
          variant="text"
          color="medium-emphasis"
          density="comfortable"
          title="View Error Logs"
          @click="expanded = !expanded"
        />
        <v-btn
          variant="text"
          color="primary"
          size="small"
          rounded="xl"
          prepend-icon="mdi-cached"
          :loading="retrying"
          @click="emit('retry', id)"
        >
          Retry
        </v-btn>
      </div>
    </div>

    <!-- Collapsible Error Stack Trace -->
    <v-expand-transition>
      <div v-if="expanded && lastError" class="error-pre-container pa-4">
        <div class="font-weight-bold mb-2">Error Log Trace:</div>
        <pre class="error-pre">{{ lastError }}</pre>
      </div>
    </v-expand-transition>
  </div>
</template>

<style scoped>
.failed-pill {
  background-color: rgb(var(--v-theme-surface-container-low));
  border-radius: 20px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.failed-main {
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
  color: rgb(var(--v-theme-error));
}

.job-type-badge.error-text {
  color: rgb(var(--v-theme-error));
}

.job-path {
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface));
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
  margin-top: 5px;
  margin-bottom: 5px;
}

.attempts {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.failed-actions {
  display: flex;
  align-items: center;
}

.error-pre-container {
  background-color: rgba(var(--v-theme-error), 0.05);
  border-top: 1px solid rgba(var(--v-theme-error), 0.1);
}

.error-pre {
  background-color: rgb(var(--v-theme-surface-container-lowest));
  color: rgb(var(--v-theme-error));
  font-family: monospace;
  font-size: 0.775rem;
  padding: 12px;
  border-radius: 12px;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
}
</style>
