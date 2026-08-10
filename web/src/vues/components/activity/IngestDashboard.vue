<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, shallowRef } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import RunningJobPill from '@/vues/components/activity/RunningJobPill.vue'
import ShowSelectedFolder from '@/vues/components/onboarding/ShowSelectedFolder.vue'
import type { JobInfo } from '@/scripts/types/api/admin.ts'
import { prettyBytes, ProcessingRateTracker } from '@/scripts/utils.ts'
import PipelineCard from '@/vues/components/activity/PipelineCard.vue'
import MdiAlertCircle from '~icons/mdi/alert-circle'
import MdiAlertCircleOutline from '~icons/mdi/alert-circle-outline'
import MdiCached from '~icons/mdi/cached'
import MdiCheckCircleOutline from '~icons/mdi/check-circle-outline'
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiClockOutline from '~icons/mdi/clock-outline'
import MdiClose from '~icons/mdi/close'
import MdiCloudUploadOutline from '~icons/mdi/cloud-upload-outline'
import MdiFileDocumentOutline from '~icons/mdi/file-document-outline'
import MdiFolderOpen from '~icons/mdi/folder-open'
import MdiFolderSearchOutline from '~icons/mdi/folder-search-outline'
import MdiImageOutline from '~icons/mdi/image-outline'
import MdiInformation from '~icons/mdi/information'
import MdiInformationOutline from '~icons/mdi/information-outline'
import MdiMagnify from '~icons/mdi/magnify'
import MdiPauseCircleOutline from '~icons/mdi/pause-circle-outline'
import MdiStop from '~icons/mdi/stop'
import MdiStopCircleOutline from '~icons/mdi/stop-circle-outline'
import MdiTrashCanOutline from '~icons/mdi/trash-can-outline'
import MdiTrayArrowUp from '~icons/mdi/tray-arrow-up'

const ingestStore = useIngestJobsStore()
const uploadStore = useUploadStore()
const authStore = useAuthStore()

// Rate tracking
const uploadRateTracker = new ProcessingRateTracker()
const metadataRateTracker = new ProcessingRateTracker()
const thumbnailsRateTracker = new ProcessingRateTracker()
const analysisRateTracker = new ProcessingRateTracker()

const uploadSpeed = ref(0)
const metadataSpeed = ref(0)
const thumbnailsSpeed = ref(0)
const analysisSpeed = ref(0)

// Scan state
const isScanning = ref(false)

// Dropzone state
const fileInput = shallowRef<HTMLInputElement | null>(null)
const folderInput = shallowRef<HTMLInputElement | null>(null)
const dragover = ref(false)

// Retrying tracking
const retryingJobIds = ref<Set<number>>(new Set())

// Detail Dialog
const detailsDialog = ref(false)
const detailedJob = ref<JobInfo | null>(null)
const isActionLoading = ref(false)

// Table Headers
const headers = computed(() => {
  return [
    { title: 'Job Type', key: 'jobType' },
    { title: 'File Path', key: 'relativePath' },
    { title: 'Attempts', key: 'attempts' },
    { title: 'Actions', key: 'actions', align: 'end' as const },
  ]
})

// Search debounce
let searchDebounce: ReturnType<typeof setTimeout> | null = null
function handleSearchInput() {
  if (searchDebounce) clearTimeout(searchDebounce)
  searchDebounce = setTimeout(() => {
    ingestStore.page = 1
    ingestStore.fetchUserJobs(true)
  }, 400)
}

function handleSearchClear() {
  ingestStore.searchQuery = ''
  ingestStore.page = 1
  ingestStore.fetchUserJobs(true)
}

// Upload actions
function triggerFileSelect() {
  fileInput.value?.click()
}

function triggerFolderSelect() {
  folderInput.value?.click()
}

function onDragOver() {
  dragover.value = true
}

function onDragLeave() {
  dragover.value = false
}

function onDrop(e: DragEvent) {
  dragover.value = false
  if (e.dataTransfer?.files?.length) {
    uploadStore.addFiles(Array.from(e.dataTransfer.files))
  }
}

function onFileChanged(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    uploadStore.addFiles(target.files)
  }
}

function onFolderChanged(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    uploadStore.addFiles(target.files)
  }
}

// Ingest progress selectors
const uploadSuccessCount = computed(
  () => uploadStore.uploads.filter((u) => u.status === 'success').length,
)
const uploadFailedCount = computed(
  () => uploadStore.uploads.filter((u) => u.status === 'failed').length,
)
const uploadTotalCount = computed(() => uploadStore.uploads.length)
const uploadToGoCount = computed(() => {
  return uploadStore.uploads.filter((u) => u.status === 'pending' || u.status === 'uploading')
    .length
})

const uploadProgress = computed(() => {
  if (uploadTotalCount.value === 0) return 100
  const completed = uploadSuccessCount.value + uploadFailedCount.value
  return Math.round((completed / uploadTotalCount.value) * 100)
})

const uploadToGoText = computed(() => {
  const active = uploadStore.activeCount
  const pending = uploadStore.uploads.filter((u) => u.status === 'pending').length
  const total = active + pending
  if (total > 0) {
    return `${total} remaining`
  }
  return 'Idle'
})

// Ingest progress indicators helper
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

// Active styles for circles
const isUploadActive = computed(() => uploadStore.isUploading)
const isMetadataActive = computed(() => computeIngestRemaining('metadata') > 0)
const isThumbnailsActive = computed(() => computeIngestRemaining('thumbnails') > 0)
const isAnalysisActive = computed(() => computeIngestRemaining('analysis') > 0)

// Track processing rate speeds
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

// Scan handlers
async function handleScan() {
  isScanning.value = true
  try {
    await ingestStore.triggerScan()
  } catch {
    // Managed in store
  } finally {
    isScanning.value = false
  }
}

// Ingest retry/cancel handlers
async function handleRetry(jobId: number) {
  retryingJobIds.value.add(jobId)
  try {
    await ingestStore.retryJob(jobId)
  } catch {
    // Managed in store
  } finally {
    retryingJobIds.value.delete(jobId)
  }
}

async function handleDialogRetry(jobId: number) {
  isActionLoading.value = true
  try {
    await ingestStore.retryJob(jobId)
    if (detailedJob.value && detailedJob.value.id === jobId) {
      detailedJob.value.status = 'queued'
    }
    detailsDialog.value = false
  } catch {
    // Managed in store
  } finally {
    isActionLoading.value = false
  }
}

function openDetails(job: JobInfo) {
  detailedJob.value = job
  detailsDialog.value = true
}

function closeDetails() {
  detailsDialog.value = false
  detailedJob.value = null
}

function getStatusColor(status: string) {
  switch (status) {
    case 'uploading':
    case 'running':
      return 'primary'
    case 'success':
    case 'done':
      return 'success'
    case 'failed':
      return 'error'
    case 'stopped':
    case 'cancelled':
      return 'warning'
    default:
      return 'grey'
  }
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'uploading':
      return MdiCloudUploadOutline
    case 'success':
    case 'done':
      return MdiCheckCircleOutline
    case 'failed':
      return MdiAlertCircleOutline
    case 'stopped':
    case 'cancelled':
      return MdiPauseCircleOutline
    default:
      return MdiClockOutline
  }
}

function formatJobType(jobType: string) {
  switch (jobType) {
    case 'ingest_metadata':
      return 'Metadata'
    case 'ingest_thumbnails':
      return 'Thumbnails'
    case 'ingest_analysis':
      return 'Analysis'
    default:
      return jobType
  }
}

function formatDate(dateStr: string | null) {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString()
}

// Watch table configurations & tabs
watch(
  () => ingestStore.page,
  () => {
    ingestStore.fetchUserJobs(true)
  },
)

watch(
  () => ingestStore.selectedTab,
  () => {
    ingestStore.page = 1
    ingestStore.fetchUserJobs(true)
  },
)

// Polling setup
onMounted(() => {
  ingestStore.startPolling(true)
  ingestStore.fetchUserJobs(true)
})

onUnmounted(() => {
  ingestStore.stopPolling(true)
})
</script>

<template>
  <div class="dashboard-root">
    <!-- Top Pipeline Section -->
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
            <v-icon size="large" :icon="MdiChevronRight" />
          </div>
        </template>
      </div>
    </section>

    <!-- Bottom Layout Grid -->
    <div class="dashboard-grid">
      <!-- Left Column: Scan, Dropzone, Upload Progress -->
      <div class="grid-column">
        <!-- File Dropzone -->
        <v-card class="action-card" flat>
          <div class="card-body">
            <h2 class="card-title">Upload Media</h2>
            <div
              class="custom-dropzone"
              :class="{ dragover: dragover }"
              @dragover.prevent="onDragOver"
              @dragleave="onDragLeave"
              @drop.prevent="onDrop"
              @click="triggerFileSelect"
            >
              <v-icon size="x-large" color="primary" class="dropzone-icon" :icon="MdiTrayArrowUp" />
              <div class="dropzone-label">
                Drag & Drop files here or <span class="browse-link">Browse</span>
              </div>
              <div class="dropzone-hint">Supports image and video file formats</div>
            </div>

            <input
              ref="fileInput"
              type="file"
              multiple
              class="hidden-input"
              @change="onFileChanged"
            />
            <input
              ref="folderInput"
              type="file"
              multiple
              webkitdirectory
              directory
              class="hidden-input"
              @change="onFolderChanged"
            />

            <div class="dropzone-buttons">
              <v-btn
                variant="tonal"
                color="secondary"
                rounded
                :prepend-icon="MdiFolderOpen"
                @click="triggerFolderSelect"
              >
                Upload Folder
              </v-btn>
              <v-btn
                v-if="uploadStore.uploads.length > 0"
                variant="plain"
                color="error"
                rounded
                :prepend-icon="MdiTrashCanOutline"
                @click="uploadStore.clearCompleted"
                class="clear-button"
              >
                Clear Completed
              </v-btn>
            </div>
          </div>
        </v-card>

        <!-- Scan Card -->
        <v-card class="action-card" flat>
          <div class="action-content">
            <div class="action-text">
              <div>
                <h2 class="section-title">Index Library Folder</h2>
                <p class="section-subtitle">
                  Folder: Start a search of your media folder to discover new photos and videos.
                </p>
              </div>
              <show-selected-folder
                class="selected-folder-display"
                bg-color="surface-variant"
                text-color="on-surface-variant"
                exclude-check-icon
                v-if="authStore.user?.mediaFolder"
                :folder="authStore.user.mediaFolder.split('/')"
                pill
              />
            </div>
            <v-btn
              color="secondary"
              variant="tonal"
              rounded="xl"
              :prepend-icon="MdiFolderSearchOutline"
              :loading="isScanning"
              @click="handleScan"
              class="scan-button"
            >
              Scan Folder
            </v-btn>
          </div>
        </v-card>

        <!-- Active Uploads List -->
        <v-card v-if="uploadStore.uploads.length > 0" class="action-card" flat>
          <div class="card-body">
            <div class="active-uploads-header">
              <h2 class="card-title">Active Uploads</h2>
              <v-btn
                variant="tonal"
                color="error"
                size="small"
                rounded
                :prepend-icon="MdiStopCircleOutline"
                @click="uploadStore.abortAll"
              >
                Cancel All
              </v-btn>
            </div>

            <div class="active-uploads-list">
              <div v-for="item in uploadStore.uploads" :key="item.id" class="upload-list-item">
                <div class="upload-item-prefix">
                  <v-progress-circular
                    v-if="item.status === 'uploading'"
                    :model-value="(item.bytesUploaded / item.size) * 100"
                    :indeterminate="item.bytesUploaded / item.size === 1"
                    color="primary"
                    size="24"
                    width="3"
                  />
                  <v-icon
                    v-else
                    :color="getStatusColor(item.status)"
                    :icon="getStatusIcon(item.status)"
                  />
                </div>

                <div class="upload-item-details">
                  <div class="upload-item-name">{{ item.name }}</div>
                  <div class="upload-item-meta">
                    {{ prettyBytes(item.bytesUploaded) }} / {{ prettyBytes(item.size) }}
                    <span v-if="item.error" class="error-text">&bull; {{ item.error }}</span>
                  </div>
                </div>

                <v-spacer />

                <div class="upload-item-actions">
                  <span class="progress-percent" v-if="item.status === 'uploading'">
                    {{ ((item.bytesUploaded / item.size) * 100).toFixed(0) }}%
                  </span>
                  <v-btn
                    v-if="item.status === 'uploading' || item.status === 'pending'"
                    color="error"
                    variant="tonal"
                    :icon="MdiStop"
                    size="x-small"
                    @click="uploadStore.stopUpload(item.id)"
                  />
                  <v-btn
                    v-else
                    color="grey"
                    variant="text"
                    :icon="MdiClose"
                    size="x-small"
                    @click="uploadStore.removeUpload(item.id)"
                  />
                </div>
              </div>
            </div>
          </div>
        </v-card>
      </div>

      <!-- Right Column: Currently Ingesting Feed & Ingest Jobs Table -->
      <div class="grid-column">
        <!-- Currently Ingesting Jobs Feed -->
        <v-card class="action-card" flat>
          <div class="card-body">
            <h2 class="feed-title">
              <span
                >Currently importing ({{ ingestStore.runningJobs.length
                }}{{ ingestStore.runningJobs.length >= 100 ? '+' : '' }})</span
              >
            </h2>

            <div class="running-list-container">
              <div v-if="ingestStore.runningJobs.length > 0" class="running-jobs-wrap">
                <RunningJobPill
                  bg-color="surface-container-high"
                  v-for="job in ingestStore.runningJobs.slice(0, 15)"
                  :key="job.id"
                  :job-type="job.jobType"
                  :relative-path="job.relativePath"
                />
                <div v-if="ingestStore.runningJobs.length > 15" class="background-tasks-indicator">
                  + {{ ingestStore.runningJobs.length - 15 }} more active tasks in background
                </div>
              </div>
              <div v-else class="running-empty-state">
                <v-icon size="large" class="empty-state-icon" :icon="MdiCheckCircleOutline" />
                <div class="empty-state-text">No active background ingestion tasks running.</div>
              </div>
            </div>
          </div>
        </v-card>

        <!-- Ingest Jobs Table with Tabs -->
        <v-card class="action-card" flat>
          <div class="card-body">
            <h2 class="card-title">Ingestion Queue Details</h2>

            <!-- Search row -->
            <div class="table-filters">
              <v-text-field
                v-model="ingestStore.searchQuery"
                label="Search filenames"
                placeholder="Search..."
                density="compact"
                variant="plain"
                rounded="xl"
                clearable
                hide-details
                :prepend-inner-icon="MdiMagnify"
                @input="handleSearchInput"
                @click:clear="handleSearchClear"
                class="search-bar"
              />
            </div>

            <v-tabs
              v-model="ingestStore.selectedTab"
              color="primary"
              class="tabs-control"
              density="comfortable"
            >
              <v-tab value="queued">Queued</v-tab>
              <v-tab value="processing">In Progress</v-tab>
              <v-tab value="failed">Failed</v-tab>
            </v-tabs>

            <!-- Table -->
            <v-data-table
              :headers="headers"
              :items="ingestStore.userJobs"
              :loading="ingestStore.isJobsLoading"
              hide-default-footer
              hover
              class="user-jobs-table"
            >
              <!-- Empty state -->
              <template #no-data>
                <div class="table-empty-state">No ingest jobs match this filter.</div>
              </template>

              <!-- Job Status Slot -->
              <template #[`item.status`]="{ item }">
                <v-chip class="table-chip">{{ item.status }}</v-chip>
              </template>

              <!-- Job Type Slot -->
              <template #[`item.jobType`]="{ item }">
                <v-chip class="table-chip">{{ formatJobType(item.jobType) }}</v-chip>
              </template>

              <!-- Relative Path Slot -->
              <template #[`item.relativePath`]="{ item }">
                <span class="path-text">
                  {{ item.relativePath || '-' }}
                </span>
              </template>

              <!-- Attempts Slot -->
              <template #[`item.attempts`]="{ item }">
                <span class="attempts-display"> {{ item.attempts }} / {{ item.maxAttempts }} </span>
              </template>

              <!-- Actions Slot -->
              <template #[`item.actions`]="{ item }">
                <div class="row-actions">
                  <!-- Retry button for failed -->
                  <v-btn
                    v-if="item.status === 'failed'"
                    :icon="MdiCached"
                    variant="tonal"
                    color="primary"
                    density="comfortable"
                    size="small"
                    :loading="retryingJobIds.has(item.id)"
                    @click="handleRetry(item.id)"
                    title="Retry Job"
                  />
                  <!-- Detail info button -->
                  <v-btn
                    :icon="MdiInformationOutline"
                    variant="text"
                    color="secondary"
                    density="comfortable"
                    size="small"
                    @click="openDetails(item)"
                    title="View Job Details"
                  />
                </div>
              </template>
            </v-data-table>

            <!-- Pagination -->
            <div
              v-if="ingestStore.totalJobsCount > ingestStore.itemsPerPage"
              class="table-pagination-row"
            >
              <span class="pagination-info">
                Showing {{ (ingestStore.page - 1) * ingestStore.itemsPerPage + 1 }} -
                {{
                  Math.min(ingestStore.page * ingestStore.itemsPerPage, ingestStore.totalJobsCount)
                }}
                of {{ ingestStore.totalJobsCount }}
              </span>
              <v-pagination
                v-model="ingestStore.page"
                :length="Math.ceil(ingestStore.totalJobsCount / ingestStore.itemsPerPage)"
                :total-visible="4"
                density="compact"
              />
            </div>
          </div>
        </v-card>
      </div>
    </div>

    <!-- Modal Dialog for Job Details -->
    <v-dialog v-model="detailsDialog" max-width="700px">
      <v-card rounded="xl" color="surface-container-highest" class="dialog-card">
        <v-card-title class="dialog-header">
          <div class="dialog-title">
            <v-icon
              :icon="detailedJob?.lastError ? MdiAlertCircle : MdiInformation"
              :color="detailedJob?.lastError ? 'error' : 'primary'"
              class="dialog-title-icon"
            />
            Job #{{ detailedJob?.id }} ({{
              detailedJob?.jobType ? formatJobType(detailedJob.jobType) : ''
            }})
          </div>
          <v-btn :icon="MdiClose" variant="text" density="comfortable" @click="closeDetails" />
        </v-card-title>

        <v-card-text class="dialog-body">
          <!-- Relative Path Info -->
          <div class="dialog-error-section">
            <div class="dialog-section-label">File Path</div>
            <code class="path-display">{{ detailedJob?.relativePath || '-' }}</code>
          </div>

          <!-- Error Stack Trace -->
          <div v-if="detailedJob?.lastError" class="dialog-error-section">
            <div class="error-label">Error Log</div>
            <pre class="console-box error-box">{{ detailedJob.lastError }}</pre>
          </div>

          <!-- Payload parameters -->
          <div>
            <div class="dialog-section-label">Parameters (Payload)</div>
            <pre class="console-box">{{ JSON.stringify(detailedJob?.payload, null, 2) }}</pre>
          </div>

          <!-- Extra Meta Info -->
          <v-row class="dialog-metadata" density="comfortable">
            <v-col cols="6" sm="4">
              <strong>Created:</strong><br />
              {{ formatDate(detailedJob?.createdAt || '') }}
            </v-col>
            <v-col cols="6" sm="4">
              <strong>Started:</strong><br />
              {{ formatDate(detailedJob?.startedAt || '') }}
            </v-col>
            <v-col cols="6" sm="4">
              <strong>Finished:</strong><br />
              {{ formatDate(detailedJob?.finishedAt || '') }}
            </v-col>
          </v-row>
        </v-card-text>

        <v-card-actions class="dialog-actions">
          <!-- Retry Button for Failed, Done, Cancelled -->
          <v-btn
            v-if="
              detailedJob &&
              (detailedJob.status === 'failed' ||
                detailedJob.status === 'done' ||
                detailedJob.status === 'cancelled')
            "
            variant="tonal"
            color="primary"
            :prepend-icon="MdiCached"
            rounded="xl"
            :loading="isActionLoading"
            @click="handleDialogRetry(detailedJob.id)"
            class="dialog-action-btn"
          >
            Retry Job
          </v-btn>
          <v-spacer />
          <v-btn color="secondary" variant="text" rounded="xl" @click="closeDetails">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.dashboard-root {
  width: 100%;
}

/* Pipeline styles */
.pipeline-section {
  border-radius: 28px;
  padding: 24px;
  margin-bottom: 32px;
}

.pipeline-row {
  display: flex;
  align-items: center;
  justify-content: space-evenly;
  flex-wrap: wrap;
  gap: 16px;
}

.pipeline-arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.5;
  color: rgb(var(--v-theme-on-surface));
}

/* Grid Layout */
.dashboard-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 28px;
}

@media (min-width: 1024px) {
  .dashboard-grid {
    grid-template-columns: 5fr 6fr;
  }
}

.grid-column {
  display: flex;
  flex-direction: column;
}

.action-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  margin-bottom: 24px;
}

.action-content {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: 20px;
}

.action-text {
  display: flex;
  align-items: flex-start;
}

.section-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
}

.section-subtitle {
  font-size: 0.875rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
  margin-top: 4px;
  margin-bottom: 0;
}

.selected-folder-display {
  margin-top: 12px;
  margin-right: 4px;
}

.scan-button {
  margin-top: 16px;
}

/* Card Body spacing */
.card-body {
  padding: 20px;
}

.card-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 12px;
}

/* Custom Dropzone */
.custom-dropzone {
  border: 2px dashed rgba(var(--v-theme-primary), 0.4);
  background-color: rgba(var(--v-theme-primary), 0.02);
  border-radius: 20px;
  padding: 36px 20px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s ease-in-out;
}

.custom-dropzone:hover,
.custom-dropzone.dragover {
  border-color: rgb(var(--v-theme-primary));
  background-color: rgba(var(--v-theme-primary), 0.06);
}

.dropzone-icon {
  margin-bottom: 8px;
}

.dropzone-label {
  font-size: 0.95rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.browse-link {
  color: rgb(var(--v-theme-primary));
  text-decoration: underline;
  font-weight: 700;
}

.dropzone-hint {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
  margin-top: 4px;
}

.hidden-input {
  display: none;
}

.dropzone-buttons {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 16px;
}

.clear-button {
  margin-left: 8px;
}

/* Active Uploads */
.active-uploads-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.active-uploads-list {
  max-height: 350px;
  overflow-y: auto;
  padding-right: 4px;
}

.upload-list-item {
  display: flex;
  align-items: center;
  padding: 12px 14px;
  background-color: rgb(var(--v-theme-surface-container-high));
  border-radius: 16px;
  margin-bottom: 8px;
  transition: background-color 0.2s ease;
}

.upload-list-item:hover {
  background-color: rgb(var(--v-theme-surface-container-highest));
}

.upload-item-prefix {
  display: flex;
  align-items: center;
}

.upload-item-details {
  margin-left: 12px;
  flex-grow: 1;
  min-width: 0;
}

.upload-item-name {
  font-weight: 600;
  font-size: 0.85rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 220px;
}

.upload-item-meta {
  font-size: 0.75rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.error-text {
  color: rgb(var(--v-theme-error));
  font-weight: 500;
}

.upload-item-actions {
  margin-left: 8px;
  display: flex;
  align-items: center;
}

.progress-percent {
  margin-right: 8px;
  font-size: 0.75rem;
  font-weight: 700;
}

/* Running Jobs Feed */
.running-list-container {
  min-height: 140px;
}

.feed-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.running-jobs-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 240px;
  overflow-y: auto;
  padding-right: 4px;
}

.background-tasks-indicator {
  text-align: center;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
  padding: 8px 0;
}

.running-empty-state {
  text-align: center;
  padding: 32px 0;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.empty-state-icon {
  opacity: 0.4;
  margin-bottom: 8px;
}

.empty-state-text {
  font-size: 0.875rem;
  font-weight: 500;
}

/* Ingest Queue Table */
.table-filters {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
  margin-bottom: 16px;
}

.search-bar {
  max-width: 280px;
}

.tabs-control {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.1);
  font-weight: 500;
  margin-bottom: 12px;
}

.user-jobs-table {
  background: transparent !important;
}

.table-empty-state {
  padding: 24px 0;
  text-align: center;
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.875rem;
}

.table-chip {
  font-weight: 500;
  font-size: 0.75rem;
}

.path-text {
  font-family: monospace;
  font-size: 0.8rem;
  color: rgb(var(--v-theme-on-surface));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: inline-block;
  max-width: 200px;
}

.attempts-display {
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.row-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
}

.table-pagination-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
}

.pagination-info {
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

/* Details Dialog */
.dialog-card {
  border: 1px solid rgba(var(--v-border-color), 0.12) !important;
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
}

.dialog-title {
  display: flex;
  align-items: center;
  font-weight: 700;
  font-size: 1.25rem;
}

.dialog-title-icon {
  margin-right: 8px;
}

.dialog-body {
  padding: 16px 24px;
}

.dialog-section-label {
  font-size: 0.875rem;
  font-weight: 700;
  color: rgba(var(--v-theme-on-surface), 0.6);
  margin-bottom: 4px;
}

.path-display {
  background-color: rgb(var(--v-theme-surface-container-lowest));
  font-family: monospace;
  font-size: 0.85rem;
  display: inline-block;
  word-break: break-all;
  padding: 4px 8px;
  border-radius: 4px;
}

.dialog-error-section {
  margin-bottom: 16px;
}

.error-label {
  color: rgb(var(--v-theme-error));
  font-size: 0.875rem;
  font-weight: 700;
  margin-bottom: 4px;
}

.console-box {
  background: rgb(var(--v-theme-surface-container-lowest));
  border: 1px solid rgba(var(--v-border-color), 0.1);
  border-radius: 12px;
  overflow-x: auto;
  font-family: monospace;
  font-size: 0.8rem;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 220px;
  overflow-y: auto;
  padding: 12px;
}

.error-box {
  color: rgb(var(--v-theme-error));
  border-color: rgba(var(--v-theme-error), 0.15);
  background-color: rgba(var(--v-theme-error), 0.02);
}

.dialog-metadata {
  margin-top: 16px;
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
}

.dialog-actions {
  padding: 16px 24px 24px 24px;
  display: flex;
  align-items: center;
  border-top: 1px solid rgba(var(--v-border-color), 0.12);
}

.dialog-action-btn {
  margin-right: 8px;
}
</style>
