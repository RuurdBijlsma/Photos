<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useIngestJobsStore } from '@/scripts/stores/ingestJobsStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { downloadBlob, filenameFromHeaders } from '@/scripts/utils.ts'
import type { JobInfo } from '@/scripts/types/api/admin.ts'
import IngestJobDetailsDialog from './IngestJobDetailsDialog.vue'
import MdiCached from '~icons/mdi/cached'
import MdiDownloadOutline from '~icons/mdi/download-outline'
import MdiInformationOutline from '~icons/mdi/information-outline'
import MdiMagnify from '~icons/mdi/magnify'

const ingestStore = useIngestJobsStore()
const snackbarStore = useSnackbarsStore()
const { isMobile } = useResponsive()

const retryingJobIds = ref<Set<number>>(new Set())
const downloadingJobIds = ref<Set<number>>(new Set())

const detailsDialog = ref(false)
const detailedJob = ref<JobInfo | null>(null)
const isActionLoading = ref(false)

const headers = computed(() => [
  { title: 'Job Type', key: 'jobType' },
  { title: 'File Path', key: 'relativePath' },
  { title: 'Attempts', key: 'attempts' },
  { title: 'Actions', key: 'actions', align: 'end' as const },
])

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

async function handleRetry(jobId: number) {
  retryingJobIds.value.add(jobId)
  try {
    await ingestStore.retryJob(jobId)
  } catch {
    // Handled in store
  } finally {
    retryingJobIds.value.delete(jobId)
  }
}

async function handleDownload(relativePath: string | null | undefined, jobId?: number) {
  if (!relativePath) return
  if (jobId !== undefined) {
    downloadingJobIds.value.add(jobId)
  }
  try {
    const response = await mediaItemService.downloadMediaFile(relativePath)
    const fallbackFilename = relativePath.split(/[/\\]/).pop() || 'download'
    const filename = filenameFromHeaders(response.headers) ?? fallbackFilename
    downloadBlob(response.data, filename)
  } catch (error) {
    snackbarStore.error(`Failed to download file: ${relativePath}`, error)
  } finally {
    if (jobId !== undefined) {
      downloadingJobIds.value.delete(jobId)
    }
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
    // Handled in store
  } finally {
    isActionLoading.value = false
  }
}

function openDetails(job: JobInfo) {
  detailedJob.value = job
  detailsDialog.value = true
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
      return jobType.replace('_', ' ')
  }
}

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
</script>

<template>
  <v-card class="action-card" flat>
    <div class="card-body">
      <h2 class="card-title">Ingestion Queue Details</h2>

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

      <div class="table-container">
        <v-data-table
          :headers="headers"
          :items="ingestStore.userJobs"
          :loading="ingestStore.isJobsLoading"
          hide-default-footer
          hover
          class="user-jobs-table"
        >
          <template #no-data>
            <div class="table-empty-state">No ingest jobs match this filter.</div>
          </template>

          <template #[`item.status`]="{ item }">
            <v-chip class="table-chip">{{ item.status }}</v-chip>
          </template>

          <template #[`item.jobType`]="{ item }">
            <v-chip class="table-chip">{{ formatJobType(item.jobType) }}</v-chip>
          </template>

          <template #[`item.relativePath`]="{ item }">
            <span class="path-text">
              {{ item.relativePath || '-' }}
            </span>
          </template>

          <template #[`item.attempts`]="{ item }">
            <span class="attempts-display"> {{ item.attempts }} / {{ item.maxAttempts }} </span>
          </template>

          <template #[`item.actions`]="{ item }">
            <div class="row-actions">
              <v-btn
                v-if="item.status === 'failed'"
                :icon="MdiCached"
                variant="text"
                color="primary"
                density="comfortable"
                size="small"
                :loading="retryingJobIds.has(item.id)"
                @click="handleRetry(item.id)"
                title="Retry Job"
              />
              <v-btn
                v-if="item.status === 'failed' && item.relativePath"
                :icon="MdiDownloadOutline"
                variant="text"
                color="secondary"
                density="comfortable"
                size="small"
                :loading="downloadingJobIds.has(item.id)"
                @click="handleDownload(item.relativePath, item.id)"
                title="Download File"
              />
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
      </div>

      <div
        v-if="ingestStore.totalJobsCount > ingestStore.itemsPerPage"
        class="table-pagination-row"
      >
        <span class="pagination-info">
          Showing {{ (ingestStore.page - 1) * ingestStore.itemsPerPage + 1 }} -
          {{ Math.min(ingestStore.page * ingestStore.itemsPerPage, ingestStore.totalJobsCount) }}
          of {{ ingestStore.totalJobsCount }}
        </span>
        <v-pagination
          v-model="ingestStore.page"
          :length="Math.ceil(ingestStore.totalJobsCount / ingestStore.itemsPerPage)"
          :total-visible="isMobile ? 3 : 4"
          density="compact"
        />
      </div>
    </div>

    <IngestJobDetailsDialog
      v-model="detailsDialog"
      :job="detailedJob"
      :is-action-loading="isActionLoading"
      :is-downloading="detailedJob ? downloadingJobIds.has(detailedJob.id) : false"
      @retry="handleDialogRetry"
      @download="handleDownload"
    />
  </v-card>
</template>

<style scoped>
.action-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  margin-bottom: 24px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.is-mobile .action-card {
  border-radius: 22px !important;
  margin-bottom: 16px;
}

.card-body {
  padding: 20px;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
}

.is-mobile .card-body {
  padding: 16px 14px;
}

.card-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 12px;
}

.is-mobile .card-title {
  font-size: 1.1rem;
}

.table-filters {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 16px;
  margin-bottom: 16px;
  width: 100%;
}

.is-mobile .table-filters {
  margin-top: 10px;
  margin-bottom: 12px;
}

.search-bar {
  max-width: 280px;
}

.is-mobile .search-bar {
  max-width: 100%;
  width: 100%;
}

.tabs-control {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.1);
  font-weight: 500;
  margin-bottom: 12px;
  width: 100%;
}

.table-container {
  width: 100%;
  max-width: 100%;
  min-width: 0;
  overflow-x: auto;
}

.user-jobs-table {
  background: transparent !important;
  width: 100%;
  max-width: 100%;
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

.is-mobile .path-text {
  max-width: 120px;
  font-size: 0.74rem;
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
  width: 100%;
}

.is-mobile .table-pagination-row {
  flex-direction: column;
  gap: 8px;
  align-items: center;
}

.pagination-info {
  font-size: 0.75rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
}
</style>
