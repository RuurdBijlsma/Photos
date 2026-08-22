<script setup lang="ts">
import MdiAlertCircleOutline from '~icons/mdi/alert-circle-outline'
import MdiDeleteForever from '~icons/mdi/delete-forever'
import MdiRefresh from '~icons/mdi/refresh'
import { computed, onMounted } from 'vue'
import { useMissingMediaStore } from '@/scripts/stores/missingMediaStore.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import { prettyBytes } from '@/scripts/utils.ts'
import { useRefreshFunction } from '@/scripts/composables/useRefreshFunction.ts'
import { useDelayedBoolean } from '@/scripts/composables/useDelayedBoolean.ts'

const missingMediaStore = useMissingMediaStore()

const flickerLoad = useDelayedBoolean(() => missingMediaStore.loading, 150)
const showLoading = computed(() => flickerLoad.value && missingMediaStore.missingItems.length === 0)

onMounted(() => {
  missingMediaStore.fetchMissing()
})

useRefreshFunction(() => missingMediaStore.fetchMissing())
</script>

<template>
  <div class="missing-settings-layout">
    <section class="config-panel">
      <v-card class="settings-card" flat border>
        <!-- Card Header -->
        <div class="card-header">
          <span class="card-title">Missing Media</span>
          <v-icon color="primary" size="large" :icon="MdiAlertCircleOutline" />
        </div>

        <div class="card-body">
          <!-- Section: Storage Resilience -->
          <div class="section-divider">
            <span class="section-label">Storage Resilience</span>
            <v-divider class="divider-line" />
          </div>

          <p class="section-desc mb-4">
            Media files that exist in your database but cannot be located on disk (e.g. disconnected
            external drive, unmounted share, or deleted files). Their metadata, thumbnails, and
            associations remain preserved until you choose to permanently prune them.
          </p>

          <!-- Action bar when items exist -->
          <div v-if="missingMediaStore.missingItems.length > 0" class="missing-actions-bar mb-4">
            <div class="missing-meta">
              <strong>{{ missingMediaStore.missingItems.length }}</strong>
              <span>
                file{{ missingMediaStore.missingItems.length === 1 ? '' : 's' }} missing &bull;
                {{ prettyBytes(missingMediaStore.totalSize) }}
              </span>
            </div>

            <div class="actions-right">
              <v-btn
                variant="text"
                color="primary"
                size="small"
                :icon="MdiRefresh"
                :loading="missingMediaStore.loading"
                v-tooltip:top="'Refresh'"
                @click="missingMediaStore.fetchMissing"
              />
              <v-btn
                color="error"
                variant="tonal"
                rounded
                size="small"
                :prepend-icon="MdiDeleteForever"
                :loading="missingMediaStore.pruning"
                @click="missingMediaStore.pruneAll"
              >
                Prune All Missing
              </v-btn>
            </div>
          </div>

          <!-- Loading state -->
          <div v-if="showLoading" class="loading-state">
            <v-progress-circular indeterminate color="primary" size="40" />
          </div>

          <!-- Timeline grid -->
          <div
            v-else-if="missingMediaStore.missingItems.length > 0"
            class="timeline-container-wrapper"
          >
            <simple-timeline
              :ideal-row-height="200"
              :timeline-items="missingMediaStore.missingItems"
              view-link="/settings/view/"
              :context="{ isMissing: true }"
              hide-drop-shadow
            />
          </div>

          <!-- Empty state -->
          <v-alert
            v-else-if="!missingMediaStore.loading"
            variant="tonal"
            type="success"
            rounded="xl"
            class="no-missing-alert"
          >
            All media files are present on disk. No missing items found.
          </v-alert>
        </div>
      </v-card>
    </section>
  </div>
</template>

<style scoped>
.missing-settings-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 24px;
}

.settings-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 24px !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.card-header {
  background-color: rgb(var(--v-theme-surface-container-high));
  padding: 16px 24px;
  border-top-left-radius: 24px;
  border-top-right-radius: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.card-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.card-body {
  padding: 24px;
}

.section-divider {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
}

.section-label {
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-primary));
  white-space: nowrap;
}

.divider-line {
  margin-left: 16px;
  opacity: 0.3;
}

.section-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
  line-height: 1.4;
}

.missing-actions-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  background-color: rgb(var(--v-theme-surface-container-high));
  border-radius: 16px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.missing-meta {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 0.9rem;
  color: rgb(var(--v-theme-on-surface));
}

.missing-meta span {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.85rem;
}

.actions-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.timeline-container-wrapper {
  height: 520px;
  width: 100%;
  overflow: hidden;
}

.loading-state {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 60px 0;
}

.no-missing-alert {
  border-radius: 16px;
}
</style>
