<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useAdminStore } from '@/scripts/stores/adminStore.ts'
import type { AdminUserInfo, JobInfo, JobStatus, JobType } from '@/scripts/types/api/admin.ts'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import { useIntervalFn, useStorage } from '@vueuse/core'

const adminStore = useAdminStore()

// Datatable parameters
const itemsPerPage = ref(10)
const tableOptions = ref({
  page: 1,
  itemsPerPage: 15,
  sortBy: [] as { key: string; order: 'asc' | 'desc' }[],
})

const autoRefresh = useStorage('autoRefreshJobsTable', false)

// Filters State
const selectedStatus = ref<JobStatus | null>('running')
const selectedJobType = ref<JobType | null>(null)
const selectedUser = ref<string | null>(null)
const searchPath = ref('')

// Detailed Inspector Modal & Loading State
const errorDialog = ref(false)
const detailedJob = ref<JobInfo | null>(null)
const isActionLoading = ref(false)

// Fetch user records if not already available to map avatars
onMounted(() => {
  if (adminStore.users.length === 0) {
    adminStore.fetchUsers()
  }
})

// Build index of users for quick lookup
const userMap = computed(() => {
  const map = new Map<number, AdminUserInfo>()
  adminStore.users.forEach((u) => map.set(u.id, u))
  return map
})

// Setup dropdown selections for filtering by registered user
const userFilterOptions = computed(() => {
  const options = adminStore.users.map((u) => ({
    title: u.username,
    value: u.id.toString(),
  }))
  options.unshift({ title: 'System', value: 'system' })
  return options
})

const headers = [
  { title: 'Job Type', key: 'jobType', sortable: true },
  { title: 'User', key: 'userId', sortable: true, width: '130px' },
  { title: 'File Path', key: 'relativePath', sortable: true },
  { title: 'Priority', key: 'priority', sortable: true, width: '100px' },
  { title: 'Status', key: 'status', sortable: true, width: '120px' },
  { title: 'Attempts', key: 'attempts', sortable: true, width: '110px' },
  { title: 'Scheduled At', key: 'scheduledAt', sortable: true },
  { title: 'Actions', key: 'actions', sortable: false, width: '80px' },
]

function triggerSearch() {
  tableOptions.value.page = 1
  loadJobs(tableOptions.value)
}

async function loadJobs(options: typeof tableOptions.value) {
  tableOptions.value = options

  const sortParams: string[] = []
  if (options.sortBy && options.sortBy.length > 0) {
    options.sortBy.forEach((s) => {
      sortParams.push(`${s.key}:${s.order}`)
    })
  }

  const filterParams: string[] = []
  if (selectedStatus.value) {
    filterParams.push(`status:eq:${selectedStatus.value}`)
  }
  if (selectedJobType.value) {
    filterParams.push(`jobType:eq:${selectedJobType.value}`)
  }
  if (selectedUser.value) {
    if (selectedUser.value === 'system') {
      filterParams.push('userId:is_null')
    } else {
      filterParams.push(`userId:eq:${selectedUser.value}`)
    }
  }
  if (searchPath.value) {
    filterParams.push(`relativePath:contains:${searchPath.value}`)
  }

  try {
    await adminStore.fetchJobs({
      page: options.page,
      limit: options.itemsPerPage,
      offset: (options.page - 1) * options.itemsPerPage,
      sort: sortParams,
      filter: filterParams,
    })
  } catch {
    autoRefresh.value = false
  }
}

function openErrorDetail(job: JobInfo) {
  detailedJob.value = job
  errorDialog.value = true
}

function closeErrorDetail() {
  errorDialog.value = false
  detailedJob.value = null
}

async function handleCancelJob(jobId: number) {
  isActionLoading.value = true
  try {
    await adminStore.cancelJob(jobId)
    await loadJobs(tableOptions.value)
    if (detailedJob.value && detailedJob.value.id === jobId) {
      detailedJob.value.status = 'cancelled'
    }
    errorDialog.value = false
  } catch {
    // Handled in Pinia Store
  } finally {
    isActionLoading.value = false
  }
}

async function handleRetryJob(jobId: number) {
  isActionLoading.value = true
  try {
    await adminStore.retryJob(jobId)
    await loadJobs(tableOptions.value)
    if (detailedJob.value && detailedJob.value.id === jobId) {
      detailedJob.value.status = 'queued'
    }
    errorDialog.value = false
  } catch {
    // Handled in Pinia Store
  } finally {
    isActionLoading.value = false
  }
}

function getStatusColor(status: JobStatus) {
  switch (status) {
    case 'done':
      return 'success'
    case 'running':
      return 'info'
    case 'queued':
      return 'warning'
    case 'failed':
      return 'error'
    case 'cancelled':
    default:
      return 'grey'
  }
}

function formatDate(dateStr: string | null) {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString()
}

function setJobType(jobType: JobType) {
  selectedJobType.value = jobType
  loadJobs(tableOptions.value)
}

const { pause, resume } = useIntervalFn(() => {
  loadJobs(tableOptions.value)
}, 1000)

watch(
  autoRefresh,
  () => {
    if (autoRefresh.value) {
      resume()
    } else {
      pause()
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="jobs-settings-layout mt-6">
    <section class="config-panel">
      <v-card class="settings-card" flat border>
        <!-- Header & Manual Refresh -->
        <div class="card-header">
          <div class="card-title-group">
            <span class="card-title">Background Job Queue</span>
            <v-icon color="primary" size="large" class="ml-2">mdi-clock-fast</v-icon>
          </div>
          <v-spacer />
          <v-btn
            v-if="!autoRefresh"
            prepend-icon="mdi-refresh"
            variant="tonal"
            color="primary"
            rounded
            :loading="adminStore.isJobsLoading"
            @click="loadJobs(tableOptions)"
          >
            Refresh Queue
          </v-btn>
          <v-switch
            hide-details
            v-model="autoRefresh"
            color="primary"
            label="Auto refresh"
            size="small"
            true-icon="mdi-check"
            false-icon="mdi-close"
            inset="material"
          ></v-switch>
        </div>

        <div class="card-body">
          <!-- Filter Bar -->
          <v-row class="mb-4" density="comfortable">
            <v-col cols="12" sm="3">
              <v-select
                v-model="selectedJobType"
                label="Filter by Job Type"
                :items="[
                  'ingest_metadata',
                  'ingest_thumbnails',
                  'ingest_analysis',
                  'ingest_llm',
                  'remove',
                  'scan',
                  'clean_db',
                  'cluster_faces',
                  'cluster_photos',
                  'import_album_item',
                  'update_global_centroid',
                  'sync_thumbnails',
                  'delayed_scan',
                  'generate_daily_cards',
                  'calc_system_stats',
                ]"
                clearable
                density="compact"
                variant="outlined"
                rounded="xl"
                hide-details
                @update:model-value="triggerSearch"
              />
            </v-col>
            <v-col cols="12" sm="3">
              <v-select
                v-model="selectedStatus"
                label="Filter by Status"
                :items="['queued', 'running', 'failed', 'done', 'cancelled']"
                clearable
                density="compact"
                variant="outlined"
                rounded="xl"
                hide-details
                @update:model-value="triggerSearch"
              />
            </v-col>
            <v-col cols="12" sm="3">
              <v-select
                v-model="selectedUser"
                label="Filter by User"
                :items="userFilterOptions"
                clearable
                density="compact"
                variant="outlined"
                rounded="xl"
                hide-details
                @update:model-value="triggerSearch"
              />
            </v-col>
            <v-col cols="12" sm="3">
              <v-text-field
                v-model="searchPath"
                label="Search Path"
                placeholder="Type and press Enter..."
                clearable
                density="compact"
                variant="outlined"
                rounded="xl"
                hide-details
                @click:clear="triggerSearch"
                @keyup.enter="triggerSearch"
              />
            </v-col>
          </v-row>

          <!-- Server Table -->
          <v-data-table-server
            v-model:items-per-page="itemsPerPage"
            :headers="headers"
            :items="adminStore.jobs"
            :items-length="adminStore.totalJobs"
            :loading="adminStore.isJobsLoading"
            item-value="id"
            multi-sort
            hover
            class="jobs-table"
            @update:options="loadJobs"
          >
            <!-- Job Type chip formatting -->
            <template #[`item.jobType`]="{ item }">
              <code @click="setJobType(item.jobType)" class="job-type-code">{{
                item.jobType
              }}</code>
            </template>

            <!-- User rendering with avatar fallback -->
            <template #[`item.userId`]="{ item }">
              <div v-if="item.userId !== null" class="d-flex align-center">
                <v-avatar size="26" class="mr-2 border" color="surface-container-highest">
                  <thumbnail-img
                    v-if="userMap.get(item.userId)?.avatarId"
                    :media-item-id="userMap.get(item.userId)!.avatarId!"
                    cover
                  />
                  <v-icon v-else size="small" color="primary">mdi-account-outline</v-icon>
                </v-avatar>
                <span class="text-caption font-weight-medium">
                  {{ userMap.get(item.userId)?.username || `ID: ${item.userId}` }}
                </span>
              </div>
              <div v-else class="d-flex align-center text-medium-emphasis">
                <v-avatar size="26" class="mr-2" color="surface-container-highest">
                  <v-icon size="small">mdi-cog-outline</v-icon>
                </v-avatar>
                <span class="text-caption italic">System</span>
              </div>
            </template>

            <!-- Relative Path formatting -->
            <template #[`item.relativePath`]="{ item }">
              <span class="path-text text-truncate d-inline-block" style="max-width: 250px">
                {{ item.relativePath || '-' }}
              </span>
            </template>

            <!-- Job Status colored chip -->
            <template #[`item.status`]="{ item }">
              <v-chip
                :color="getStatusColor(item.status)"
                size="small"
                variant="tonal"
                class="font-weight-medium"
              >
                {{ item.status }}
              </v-chip>
            </template>

            <!-- Attempts progress formatting -->
            <template #[`item.attempts`]="{ item }">
              <span class="text-caption text-medium-emphasis">
                {{ item.attempts }} / {{ item.maxAttempts }}
              </span>
            </template>

            <!-- Timestamps formatted locally -->
            <template #[`item.scheduledAt`]="{ item }">
              <span class="text-caption">
                {{ formatDate(item.scheduledAt) }}
              </span>
            </template>

            <!-- Row inspector action -->
            <template #[`item.actions`]="{ item }">
              <v-btn
                v-if="item.lastError"
                icon="mdi-alert-circle-outline"
                variant="text"
                color="error"
                density="comfortable"
                title="View Error Details"
                @click="openErrorDetail(item)"
              />
              <v-btn
                v-else
                icon="mdi-information-outline"
                variant="text"
                color="primary"
                density="comfortable"
                title="View Payload Details"
                @click="openErrorDetail(item)"
              />
            </template>
          </v-data-table-server>
        </div>
      </v-card>
    </section>

    <!-- Modal: Job Details & Error Inspector -->
    <v-dialog v-model="errorDialog" max-width="800px">
      <v-card rounded="xl" color="surface-container-highest" class="border">
        <v-card-title class="dialog-header d-flex align-center justify-space-between py-4 px-6">
          <div class="d-flex align-center font-weight-bold">
            <v-icon
              :icon="detailedJob?.lastError ? 'mdi-alert-circle' : 'mdi-information'"
              :color="detailedJob?.lastError ? 'error' : 'primary'"
              class="mr-2"
            />
            Job #{{ detailedJob?.id }} ({{ detailedJob?.jobType }})
          </div>
          <v-btn icon="mdi-close" variant="text" density="comfortable" @click="closeErrorDetail" />
        </v-card-title>

        <v-card-text class="py-4 px-6 overflow-y-auto" style="max-height: 60vh">
          <!-- Error Stack Trace -->
          <div v-if="detailedJob?.lastError" class="mb-4">
            <div class="text-subtitle-2 font-weight-bold error-title mb-2">Error Log</div>
            <pre class="error-console pa-4">{{ detailedJob.lastError }}</pre>
          </div>

          <!-- Payload parameters -->
          <div>
            <div class="text-subtitle-2 font-weight-bold mb-2 text-medium-emphasis">
              Job Parameters (Payload)
            </div>
            <pre class="payload-console pa-4">{{
              JSON.stringify(detailedJob?.payload, null, 2)
            }}</pre>
          </div>

          <!-- Extra Meta Info -->
          <v-row class="mt-4 text-caption text-medium-emphasis" density="comfortable">
            <v-col cols="6" sm="3">
              <strong>Created:</strong><br />
              {{ formatDate(detailedJob?.createdAt || '') }}
            </v-col>
            <v-col cols="6" sm="3">
              <strong>Started:</strong><br />
              {{ formatDate(detailedJob?.startedAt || '') }}
            </v-col>
            <v-col cols="6" sm="3">
              <strong>Finished:</strong><br />
              {{ formatDate(detailedJob?.finishedAt || '') }}
            </v-col>
            <v-col cols="6" sm="3">
              <strong>Worker Host:</strong><br />
              {{ detailedJob?.owner || 'Unassigned' }}
            </v-col>
          </v-row>
        </v-card-text>

        <v-card-actions class="px-6 pb-6 d-flex align-center">
          <!-- Cancel Button: Queued or Running -->
          <v-btn
            v-if="
              detailedJob && (detailedJob.status === 'queued' || detailedJob.status === 'running')
            "
            variant="tonal"
            prepend-icon="mdi-stop-circle-outline"
            rounded
            :loading="isActionLoading"
            @click="handleCancelJob(detailedJob.id)"
            class="mr-2"
          >
            Cancel Job
          </v-btn>

          <!-- Retry Button: Failed, Done, Cancelled -->
          <v-btn
            v-if="
              detailedJob &&
              (detailedJob.status === 'failed' ||
                detailedJob.status === 'done' ||
                detailedJob.status === 'cancelled')
            "
            variant="tonal"
            prepend-icon="mdi-cached"
            rounded
            :loading="isActionLoading"
            @click="handleRetryJob(detailedJob.id)"
            class="mr-2"
          >
            Retry Job
          </v-btn>

          <v-spacer />
          <v-btn color="primary" variant="text" rounded @click="closeErrorDetail">Close</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
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
  gap: 25px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.card-title-group {
  display: flex;
  align-items: center;
}

.card-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.card-body {
  padding: 24px;
}

.jobs-table {
  background: transparent !important;
}

.job-type-code {
  background-color: rgba(var(--v-theme-primary), 0.1);
  color: rgb(var(--v-theme-primary));
  padding: 2px 8px;
  border-radius: 6px;
  font-family: monospace;
  font-size: 0.85rem;
  cursor: pointer;
}

.path-text {
  font-family: monospace;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface));
}

.error-title {
  color: rgb(var(--v-theme-error));
}

.error-console,
.payload-console {
  background: rgb(var(--v-theme-surface-container-lowest));
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 12px;
  overflow-x: auto;
  font-family: monospace;
  font-size: 0.825rem;
  white-space: pre-wrap;
  word-break: break-all;
}

.error-console {
  color: rgb(var(--v-theme-error));
  border-color: rgba(var(--v-theme-error), 0.15);
  background-color: rgba(var(--v-theme-error), 0.02);
}

.dialog-header {
  border-bottom: 1px solid rgba(var(--v-border-color), 0.15);
}

.italic {
  font-style: italic;
}
</style>
