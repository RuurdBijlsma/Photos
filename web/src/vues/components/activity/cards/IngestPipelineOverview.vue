<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import PipelineCard from '@/vues/components/activity/PipelineCard.vue'
import { ProcessingRateTracker } from '@/scripts/utils.ts'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiCloudUploadOutline from '~icons/mdi/cloud-upload-outline'
import MdiFileDocumentOutline from '~icons/mdi/file-document-outline'
import MdiImageOutline from '~icons/mdi/image-outline'
import MdiMagnify from '~icons/mdi/magnify'

const ingestStore = useIngestJobsStore()
const uploadStore = useUploadStore()
const { isMobile } = useResponsive()

// Rate tracking
const uploadRateTracker = new ProcessingRateTracker()
const metadataRateTracker = new ProcessingRateTracker()
const thumbnailsRateTracker = new ProcessingRateTracker()
const analysisRateTracker = new ProcessingRateTracker()

const uploadSpeed = ref(0)
const metadataSpeed = ref(0)
const thumbnailsSpeed = ref(0)
const analysisSpeed = ref(0)

// Ingest progress selectors
const uploadSuccessCount = computed(() => uploadStore.successCount)
const uploadFailedCount = computed(() => uploadStore.failedCount)
const uploadTotalCount = computed(() => uploadStore.totalCount)
const uploadToGoCount = computed(() => uploadStore.activeCount + uploadStore.pendingCount)

const uploadProgress = computed(() => {
  if (uploadTotalCount.value === 0) return 100
  const completed = uploadSuccessCount.value + uploadFailedCount.value
  return Math.round((completed / uploadTotalCount.value) * 100)
})

const uploadToGoText = computed(() => {
  const total = uploadStore.activeCount + uploadStore.pendingCount
  if (total > 0) {
    return `${total} remaining`
  }
  return 'Idle'
})

function computeIngestProgress(category: 'metadata' | 'thumbnails' | 'analysis') {
  const counts = ingestStore.overview?.[category]
  if (!counts || counts.total === 0) return 100
  const done = counts.total - counts.queued - counts.running
  return Math.round((done / counts.total) * 100)
}

function computeIngestRemaining(category: 'metadata' | 'thumbnails' | 'analysis') {
  const counts = ingestStore.overview?.[category]
  if (!counts) return 0
  return counts.queued + counts.running
}

const metadataProgress = computed(() => computeIngestProgress('metadata'))
const metadataToGoText = computed(() => {
  const remaining = computeIngestRemaining('metadata')
  return remaining > 0 ? `${remaining} to go` : 'Done'
})

const thumbnailsProgress = computed(() => computeIngestProgress('thumbnails'))
const thumbnailsToGoText = computed(() => {
  const remaining = computeIngestRemaining('thumbnails')
  return remaining > 0 ? `${remaining} to go` : 'Done'
})

const analysisProgress = computed(() => computeIngestProgress('analysis'))
const analysisToGoText = computed(() => {
  const remaining = computeIngestRemaining('analysis')
  return remaining > 0 ? `${remaining} to go` : 'Done'
})

const isUploadActive = computed(() => uploadStore.isUploading)
const isMetadataActive = computed(() => computeIngestRemaining('metadata') > 0)
const isThumbnailsActive = computed(() => computeIngestRemaining('thumbnails') > 0)
const isAnalysisActive = computed(() => computeIngestRemaining('analysis') > 0)

watch(
  () => uploadSuccessCount.value + uploadFailedCount.value,
  (completed) => {
    uploadSpeed.value = uploadRateTracker.update(completed)
  },
  { immediate: true },
)

watch(
  () => ingestStore.overview,
  (overview) => {
    if (!overview) return
    metadataSpeed.value = metadataRateTracker.update(overview.metadata?.done ?? 0)
    thumbnailsSpeed.value = thumbnailsRateTracker.update(overview.thumbnails?.done ?? 0)
    analysisSpeed.value = analysisRateTracker.update(overview.analysis?.done ?? 0)
  },
  { deep: true, immediate: true },
)

const pipelineSteps = computed(() => [
  {
    key: 'upload',
    label: 'Upload',
    icon: MdiCloudUploadOutline,
    progress: uploadProgress.value,
    statusText: uploadToGoText.value,
    isActive: isUploadActive.value,
    toGo: uploadToGoCount.value,
    speed: uploadSpeed.value,
    tooltip:
      uploadTotalCount.value > 0
        ? `${uploadSuccessCount.value + uploadFailedCount.value} / ${uploadTotalCount.value}`
        : '',
  },
  {
    key: 'metadata',
    label: 'Metadata',
    icon: MdiFileDocumentOutline,
    progress: metadataProgress.value,
    statusText: metadataToGoText.value,
    isActive: isMetadataActive.value,
    toGo: computeIngestRemaining('metadata'),
    speed: metadataSpeed.value,
    tooltip: '',
  },
  {
    key: 'thumbnails',
    label: 'Thumbnails',
    icon: MdiImageOutline,
    progress: thumbnailsProgress.value,
    statusText: thumbnailsToGoText.value,
    isActive: isThumbnailsActive.value,
    toGo: computeIngestRemaining('thumbnails'),
    speed: thumbnailsSpeed.value,
    tooltip: '',
  },
  {
    key: 'analysis',
    label: 'Analysis',
    icon: MdiMagnify,
    progress: analysisProgress.value,
    statusText: analysisToGoText.value,
    isActive: isAnalysisActive.value,
    toGo: computeIngestRemaining('analysis'),
    speed: analysisSpeed.value,
    tooltip: '',
  },
])
</script>

<template>
  <section class="pipeline-section">
    <div class="pipeline-row">
      <template v-for="(step, index) in pipelineSteps" :key="step.key">
        <pipeline-card
          :label="step.label"
          :icon="step.icon"
          :progress="step.progress"
          :status="step.statusText"
          :is-active="step.isActive"
          :to-go="step.toGo"
          :items-per-second="step.speed"
          :tooltip-text="step.tooltip"
        />

        <div v-if="index < pipelineSteps.length - 1" class="pipeline-arrow">
          <v-icon :size="isMobile ? 'default' : 'large'" :icon="MdiChevronRight" />
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.pipeline-section {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
  border-radius: 28px;
  padding: 24px;
  margin-bottom: 32px;
}

.is-mobile .pipeline-section {
  padding: 0;
  margin-bottom: 20px;
}

.pipeline-row {
  display: flex;
  align-items: center;
  justify-content: space-evenly;
  flex-wrap: wrap;
  gap: 16px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
}

.is-mobile .pipeline-row {
  flex-wrap: nowrap;
  justify-content: flex-start;
  overflow-x: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
  padding: 4px 4px 10px 4px;
  gap: 8px;
  scroll-snap-type: x mandatory;
  -webkit-overflow-scrolling: touch;
}

.is-mobile .pipeline-row::-webkit-scrollbar {
  display: none;
}

.pipeline-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.5;
  color: rgb(var(--v-theme-on-surface));
  flex-shrink: 0;
}

.is-mobile .pipeline-arrow {
  opacity: 0.35;
}
</style>
