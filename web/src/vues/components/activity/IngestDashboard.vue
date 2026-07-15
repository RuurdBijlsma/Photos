<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useUploadStore } from '@/scripts/stores/uploadStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import RunningJobPill from '@/vues/components/activity/RunningJobPill.vue'
import ShowSelectedFolder from '@/vues/components/onboarding/ShowSelectedFolder.vue'
import type { JobInfo } from '@/scripts/types/api/admin.ts'
import { prettyBytes } from '@/scripts/utils.ts'

const ingestStore = useIngestJobsStore()
const uploadStore = useUploadStore()
const authStore = useAuthStore()

// Scan state
const isScanning = ref(false)

// Dropzone state
const fileInput = ref<HTMLInputElement | null>(null)
const folderInput = ref<HTMLInputElement | null>(null)
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
      return 'mdi-cloud-upload-outline'
    case 'success':
    case 'done':
      return 'mdi-check-circle-outline'
    case 'failed':
      return 'mdi-alert-circle-outline'
    case 'stopped':
    case 'cancelled':
      return 'mdi-pause-circle-outline'
    default:
      return 'mdi-clock-outline'
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
    <section class="pipeline-section mb-8">
      <div class="pipeline-row">
        <!-- Upload circle -->
        <div
          class="pipeline-step-wrapper"
          v-tooltip="{
            location: 'top',
            text: `${uploadSuccessCount + uploadFailedCount} / ${uploadTotalCount}`,
            disabled: uploadTotalCount === 0,
          }"
        >
          <div class="pipeline-step-card" :class="{ active: isUploadActive }">
            <v-progress-circular
              :model-value="uploadProgress"
              color="primary"
              size="88"
              width="6"
              class="pipeline-circle"
            >
              <div class="circle-inner">
                <v-icon size="default">mdi-cloud-upload-outline</v-icon>
                <span class="circle-pct">{{ uploadProgress }}%</span>
              </div>
            </v-progress-circular>
            <div class="step-details mt-2">
              <span class="step-label font-weight-bold">Upload</span>
              <span class="step-status text-caption text-medium-emphasis">{{
                uploadToGoText
              }}</span>
            </div>
          </div>
        </div>

        <div class="pipeline-arrow">
          <v-icon size="large">mdi-chevron-right</v-icon>
        </div>

        <!-- Metadata circle -->
        <div class="pipeline-step-wrapper">
          <div class="pipeline-step-card" :class="{ active: isMetadataActive }">
            <v-progress-circular
              :model-value="metadataProgress"
              color="primary"
              size="88"
              width="6"
              class="pipeline-circle"
            >
              <div class="circle-inner">
                <v-icon size="default">mdi-file-document-outline</v-icon>
                <span class="circle-pct">{{ metadataProgress }}%</span>
              </div>
            </v-progress-circular>
            <div class="step-details mt-2">
              <span class="step-label font-weight-bold">Metadata</span>
              <span class="step-status text-caption text-medium-emphasis">{{
                metadataToGoText
              }}</span>
            </div>
          </div>
        </div>

        <div class="pipeline-arrow">
          <v-icon size="large">mdi-chevron-right</v-icon>
        </div>

        <!-- Thumbnails circle -->
        <div class="pipeline-step-wrapper">
          <div class="pipeline-step-card" :class="{ active: isThumbnailsActive }">
            <v-progress-circular
              :model-value="thumbnailsProgress"
              color="primary"
              size="88"
              width="6"
              class="pipeline-circle"
            >
              <div class="circle-inner">
                <v-icon size="default">mdi-image-outline</v-icon>
                <span class="circle-pct">{{ thumbnailsProgress }}%</span>
              </div>
            </v-progress-circular>
            <div class="step-details mt-2">
              <span class="step-label font-weight-bold">Thumbnails</span>
              <span class="step-status text-caption text-medium-emphasis">{{
                thumbnailsToGoText
              }}</span>
            </div>
          </div>
        </div>

        <div class="pipeline-arrow">
          <v-icon size="large">mdi-chevron-right</v-icon>
        </div>

        <!-- Analysis circle -->
        <div class="pipeline-step-wrapper">
          <div class="pipeline-step-card" :class="{ active: isAnalysisActive }">
            <v-progress-circular
              :model-value="analysisProgress"
              color="primary"
              size="88"
              width="6"
              class="pipeline-circle"
            >
              <div class="circle-inner">
                <v-icon size="default">mdi-magnify</v-icon>
                <span class="circle-pct">{{ analysisProgress }}%</span>
              </div>
            </v-progress-circular>
            <div class="step-details mt-2">
              <span class="step-label font-weight-bold">Analysis</span>
              <span class="step-status text-caption text-medium-emphasis">{{
                analysisToGoText
              }}</span>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Bottom Layout Grid -->
    <div class="dashboard-grid">
      <!-- Left Column: Scan, Dropzone, Upload Progress -->
      <div class="grid-column">
        <!-- Scan Card -->
        <v-card class="action-card mb-6" flat>
          <div class="action-content pa-5">
            <div class="action-text">
              <div>
                <h2 class="fix-margin text-h6 font-weight-bold">Index Library Folder</h2>
                <p class="text-subtitle-2 text-medium-emphasis mb-0 mt-1">
                  Folder: Start a search of your media folder to discover new photos and videos.
                </p>
              </div>
              <show-selected-folder
                class="mt-3 mr-1"
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
              prepend-icon="mdi-folder-search-outline"
              :loading="isScanning"
              @click="handleScan"
              class="mt-4"
            >
              Scan Folder
            </v-btn>
          </div>
        </v-card>

        <!-- Simplified File Dropzone -->
        <v-card class="action-card mb-6" flat>
          <div class="pa-5">
            <h2 class="text-h6 font-weight-bold mb-3">Upload Media</h2>
            <div
              class="custom-dropzone"
              :class="{ dragover: dragover }"
              @dragover.prevent="onDragOver"
              @dragleave="onDragLeave"
              @drop.prevent="onDrop"
              @click="triggerFileSelect"
            >
              <v-icon size="x-large" color="primary" class="mb-2">mdi-tray-arrow-up</v-icon>
              <div class="dropzone-label">
                Drag & Drop files here or <span class="browse-link">Browse</span>
              </div>
              <div class="dropzone-hint text-caption text-medium-emphasis mt-1">
                Supports image and video file formats
              </div>
            </div>

            <input ref="fileInput" type="file" multiple class="d-none" @change="onFileChanged" />
            <input
              ref="folderInput"
              type="file"
              multiple
              webkitdirectory
              directory
              class="d-none"
              @change="onFolderChanged"
            />

            <div class="dropzone-buttons mt-4">
              <v-btn
                variant="tonal"
                color="secondary"
                rounded
                prepend-icon="mdi-folder-open"
                @click="triggerFolderSelect"
              >
                Upload Folder
              </v-btn>
              <v-btn
                v-if="uploadStore.uploads.length > 0"
                variant="plain"
                color="error"
                rounded
                prepend-icon="mdi-trash-can-outline"
                @click="uploadStore.clearCompleted"
                class="ml-2"
              >
                Clear Completed
              </v-btn>
            </div>
          </div>
        </v-card>

        <!-- Active Uploads List -->
        <v-card v-if="uploadStore.uploads.length > 0" class="action-card" flat>
          <div class="pa-5">
            <div class="d-flex align-center justify-space-between mb-4">
              <h2 class="text-h6 font-weight-bold">Active Uploads</h2>
              <v-btn
                variant="tonal"
                color="error"
                size="small"
                rounded
                prepend-icon="mdi-stop-circle-outline"
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
                  <v-icon v-else :color="getStatusColor(item.status)">
                    {{ getStatusIcon(item.status) }}
                  </v-icon>
                </div>

                <div class="upload-item-details ml-3">
                  <div class="upload-item-name">{{ item.name }}</div>
                  <div class="upload-item-meta text-caption text-medium-emphasis">
                    {{ prettyBytes(item.bytesUploaded) }} / {{ prettyBytes(item.size) }}
                    <span v-if="item.error" class="text-error font-weight-medium"
                      >&bull; {{ item.error }}</span
                    >
                  </div>
                </div>

                <v-spacer />

                <div class="upload-item-actions ml-2">
                  <span
                    class="progress-percent mr-2 text-caption font-weight-bold"
                    v-if="item.status === 'uploading'"
                  >
                    {{ ((item.bytesUploaded / item.size) * 100).toFixed(0) }}%
                  </span>
                  <v-btn
                    v-if="item.status === 'uploading' || item.status === 'pending'"
                    color="error"
                    variant="tonal"
                    icon="mdi-stop"
                    size="x-small"
                    @click="uploadStore.stopUpload(item.id)"
                  />
                  <v-btn
                    v-else
                    color="grey"
                    variant="text"
                    icon="mdi-close"
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
        <v-card class="action-card mb-6" flat>
          <div class="pa-5">
            <h2 class="text-h6 font-weight-bold mb-3 d-flex align-center gap-2">
              <span
                >Currently importing ({{ ingestStore.runningJobs.length
                }}{{ ingestStore.runningJobs.length >= 100 ? '+' : '' }})</span
              >
            </h2>

            <div class="running-list-container">
              <div v-if="ingestStore.runningJobs.length > 0" class="running-jobs-wrap">
                <RunningJobPill
                  bg-color="surface-container-high"
                  v-for="job in ingestStore.runningJobs"
                  :key="job.id"
                  :job-type="job.jobType"
                  :relative-path="job.relativePath"
                />
              </div>
              <div v-else class="running-empty-state text-center py-8 text-medium-emphasis">
                <v-icon size="large" class="opacity-40 mb-2">mdi-check-circle-outline</v-icon>
                <div class="text-subtitle-2">No active background ingestion tasks running.</div>
              </div>
            </div>
          </div>
        </v-card>

        <!-- Ingest Jobs Table with Tabs -->
        <v-card class="action-card" flat>
          <div class="pa-5">
            <h2 class="text-h6 font-weight-bold mb-4">Ingestion Queue Details</h2>

            <!-- Search row -->
            <div class="table-filters mb-4 mt-4">
              <v-text-field
                v-model="ingestStore.searchQuery"
                label="Search filenames"
                placeholder="Search..."
                density="compact"
                variant="plain"
                rounded="xl"
                clearable
                hide-details
                prepend-inner-icon="mdi-magnify"
                @input="handleSearchInput"
                @click:clear="handleSearchClear"
                class="search-bar"
              />
            </div>

            <v-tabs
              v-model="ingestStore.selectedTab"
              color="primary"
              class="tabs-control mb-3"
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
                <div class="py-6 text-center text-medium-emphasis text-body-2">
                  No ingest jobs match this filter.
                </div>
              </template>

              <!-- Job Type Slot -->
              <template #[`item.status`]="{ item }">
                <v-chip class="font-weight-medium text-caption">{{ item.status }}</v-chip>
              </template>

              <!-- Job Type Slot -->
              <template #[`item.jobType`]="{ item }">
                <v-chip class="font-weight-medium text-caption">{{
                  formatJobType(item.jobType)
                }}</v-chip>
              </template>

              <!-- Relative Path Slot -->
              <template #[`item.relativePath`]="{ item }">
                <span class="path-text text-truncate d-inline-block" style="max-width: 200px">
                  {{ item.relativePath || '-' }}
                </span>
              </template>

              <!-- Attempts Slot -->
              <template #[`item.attempts`]="{ item }">
                <span class="text-caption text-medium-emphasis"
                  >{{ item.attempts }} / {{ item.maxAttempts }}</span
                >
              </template>

              <!-- Actions Slot -->
              <template #[`item.actions`]="{ item }">
                <div class="d-flex align-center justify-end gap-1">
                  <!-- Retry button for failed -->
                  <v-btn
                    v-if="item.status === 'failed'"
                    icon="mdi-cached"
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
                    icon="mdi-information-outline"
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
              class="d-flex align-center justify-space-between mt-4"
            >
              <span class="text-caption text-medium-emphasis">
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
        <v-card-title
          class="dialog-header d-flex align-center justify-space-between py-4 px-6 border-bottom"
        >
          <div class="d-flex align-center font-weight-bold text-h6">
            <v-icon
              :icon="detailedJob?.lastError ? 'mdi-alert-circle' : 'mdi-information'"
              :color="detailedJob?.lastError ? 'error' : 'primary'"
              class="mr-2"
            />
            Job #{{ detailedJob?.id }} ({{
              detailedJob?.jobType ? formatJobType(detailedJob.jobType) : ''
            }})
          </div>
          <v-btn icon="mdi-close" variant="text" density="comfortable" @click="closeDetails" />
        </v-card-title>

        <v-card-text class="py-4 px-6 dialog-body">
          <!-- Relative Path Info -->
          <div class="mb-4">
            <div class="text-subtitle-2 font-weight-bold text-medium-emphasis mb-1">File Path</div>
            <code class="path-display px-2 py-1 rounded">{{
              detailedJob?.relativePath || '-'
            }}</code>
          </div>

          <!-- Error Stack Trace -->
          <div v-if="detailedJob?.lastError" class="mb-4">
            <div class="text-subtitle-2 font-weight-bold error-label mb-1">Error Log</div>
            <pre class="console-box error-box pa-3">{{ detailedJob.lastError }}</pre>
          </div>

          <!-- Payload parameters -->
          <div>
            <div class="text-subtitle-2 font-weight-bold text-medium-emphasis mb-1">
              Parameters (Payload)
            </div>
            <pre class="console-box pa-3">{{ JSON.stringify(detailedJob?.payload, null, 2) }}</pre>
          </div>

          <!-- Extra Meta Info -->
          <v-row class="mt-4 text-caption text-medium-emphasis" density="comfortable">
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

        <v-card-actions class="px-6 pb-6 d-flex align-center border-top pt-4">
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
            prepend-icon="mdi-cached"
            rounded="xl"
            :loading="isActionLoading"
            @click="handleDialogRetry(detailedJob.id)"
            class="mr-2"
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
}

.pipeline-row {
  display: flex;
  align-items: center;
  justify-content: space-evenly;
  flex-wrap: wrap;
  gap: 16px;
}

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
}

.step-label {
  font-size: 0.9rem;
  color: rgb(var(--v-theme-on-surface));
}

.step-status {
  font-weight: 500;
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
}

.action-content {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.action-text {
  display: flex;
  align-items: flex-start;
}

.fix-margin {
  margin: 0;
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
  font-weight: 500;
}

.dropzone-buttons {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

/* Active Uploads */
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

.upload-item-name {
  font-weight: 600;
  font-size: 0.85rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 220px;
}

.upload-item-meta {
  font-weight: 500;
  opacity: 0.8;
}

/* Running Jobs Feed */
.running-list-container {
  min-height: 140px;
}

.running-jobs-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 240px;
  overflow-y: auto;
  padding-right: 4px;
}

/* Ingest Queue Table */
.table-filters {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.search-bar {
  max-width: 280px;
}

.tabs-control {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.1);
  font-weight: 500;
}

.user-jobs-table {
  background: transparent !important;
}

.path-text {
  font-family: monospace;
  font-size: 0.8rem;
  color: rgb(var(--v-theme-on-surface));
}

.gap-1 {
  gap: 4px;
}

.gap-2 {
  gap: 8px;
}

/* Details Dialog */
.dialog-card {
  border: 1px solid rgba(var(--v-border-color), 0.12) !important;
}

.dialog-header {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.12);
}

.path-display {
  background-color: rgb(var(--v-theme-surface-container-lowest));
  font-family: monospace;
  font-size: 0.85rem;
  display: inline-block;
  word-break: break-all;
}

.error-label {
  color: rgb(var(--v-theme-error));
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
}

.error-box {
  color: rgb(var(--v-theme-error));
  border-color: rgba(var(--v-theme-error), 0.15);
  background-color: rgba(var(--v-theme-error), 0.02);
}

.border-top {
  border-top: 1px solid rgba(var(--v-border-color), 0.12);
}
</style>
