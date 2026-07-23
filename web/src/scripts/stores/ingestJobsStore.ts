import { ref, type Ref } from 'vue'
import { defineStore } from 'pinia'
import type { IngestOverviewResponse } from '@/scripts/types/api/ingestJobs.ts'
import type { JobInfo } from '@/scripts/types/api/admin.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import ingestJobsService from '@/scripts/services/ingestJobsService.ts'

interface DisplayJob extends JobInfo {
  _displayFinishedTime?: number
}

export const useIngestJobsStore = defineStore('ingestJobs', () => {
  const snackbarStore = useSnackbarsStore()

  // --- CONFIGURATION ---
  const MAX_DISPLAY_ITEMS = 100

  // --- STATE ---
  const overview: Ref<IngestOverviewResponse | null> = ref(null)
  const runningJobs: Ref<DisplayJob[]> = ref([])
  const userJobs: Ref<JobInfo[]> = ref([])

  const isOverviewLoading: Ref<boolean> = ref(false)
  const isRunningLoading: Ref<boolean> = ref(false)
  const isJobsLoading = ref(false)

  // Paginated Jobs Table State
  const totalJobsCount = ref(0)
  const page = ref(1)
  const itemsPerPage = ref(10)
  const searchQuery = ref('')
  const selectedTab = ref('queued') // queued, processing, failed

  // Polling State & Connection Counters
  let pollingIntervalId: ReturnType<typeof setInterval> | null = null
  const activeSubscribers = ref(0)
  const needsDetailsScope = ref(0)

  // Plain JavaScript Array to avoid Vue reactivity proxying on internal queue operations
  const trickleQueue: DisplayJob[] = []
  const targetRate = ref(0) // Target flow rate in jobs per millisecond [1]
  const accumulator = ref(0) // Accumulates fractional jobs over time [1]
  let lastTickTime = Date.now() // Tracks absolute timestamp of last loop cycle [1]
  let trickleIntervalId: ReturnType<typeof setInterval> | null = null

  // --- PRIVATE UTILS ---

  function processIncomingJobs(incomingJobs: DisplayJob[]) {
    const incomingMap = new Map<number, DisplayJob>()
    for (const job of incomingJobs) {
      incomingMap.set(job.id, job)
    }

    // 1. Update status on items already being shown in the list
    runningJobs.value = runningJobs.value.map((existingJob) => {
      const incoming = incomingMap.get(existingJob.id)
      if (incoming) {
        const becameDone = existingJob.status !== 'done' && incoming.status === 'done'
        const updated = { ...existingJob, ...incoming }
        if (becameDone) {
          updated._displayFinishedTime = Date.now() // Lock in completion timestamp [1]
        }
        return updated
      } else {
        // Fallback safety: transition jobs that disappeared during polling gaps to 'done'
        if (existingJob.status === 'running') {
          return {
            ...existingJob,
            status: 'done',
            _displayFinishedTime: Date.now(),
          }
        }
        return existingJob
      }
    })

    // 2. Identify brand-new items using the plain trickleQueue array
    const existingIds = new Set(runningJobs.value.map((j) => j.id))
    const queuedIds = new Set(trickleQueue.map((j) => j.id))

    const newJobs = incomingJobs.filter((job) => !existingIds.has(job.id) && !queuedIds.has(job.id))

    if (newJobs.length > 0) {
      trickleQueue.push(...newJobs)
    }

    // 3. Recalculate dynamic flow rate
    if (trickleQueue.length > 0) {
      targetRate.value = trickleQueue.length / 3000
    } else {
      targetRate.value = 0
      accumulator.value = 0
    }
  }

  function tickTrickleAndCleanup() {
    const now = Date.now()
    const dt = now - lastTickTime
    lastTickTime = now

    let updated = false
    // Working copy to batch updates and prevent intermediate reactive triggers
    const currentJobs = [...runningJobs.value]

    // 1. Accumulate fractional items based on elapsed time and release completed integers
    if (trickleQueue.length > 0 && targetRate.value > 0) {
      accumulator.value += dt * targetRate.value
      const itemsToAdd = Math.floor(accumulator.value)

      if (itemsToAdd > 0) {
        accumulator.value -= itemsToAdd

        for (let i = 0; i < itemsToAdd; i++) {
          const nextJob = trickleQueue.shift()
          if (nextJob) {
            if (nextJob.status === 'done' && !nextJob._displayFinishedTime) {
              nextJob._displayFinishedTime = now
            }
            currentJobs.unshift(nextJob)
            updated = true
          }
        }
      }
    } else {
      accumulator.value = 0
      targetRate.value = 0
    }

    // 2. Cleanup age-out: Optimize by checking if there is anything to clean up first
    const hasDoneJobs = currentJobs.some((job) => job.status === 'done')
    let jobsFiltered = currentJobs

    if (hasDoneJobs) {
      jobsFiltered = currentJobs.filter((job) => {
        if (job.status !== 'done') {
          return true
        }
        const finishedTime = job._displayFinishedTime || now
        return now - finishedTime <= 5000
      })
      if (jobsFiltered.length !== currentJobs.length) {
        updated = true
      }
    }

    // 3. Performance safety cap
    if (jobsFiltered.length > MAX_DISPLAY_ITEMS) {
      let excessCount = jobsFiltered.length - MAX_DISPLAY_ITEMS
      for (let i = jobsFiltered.length - 1; i >= 0 && excessCount > 0; i--) {
        if (jobsFiltered[i].status === 'done') {
          jobsFiltered.splice(i, 1)
          excessCount--
          updated = true
        }
      }
    }

    // 4. Batch updates: Only trigger Vue reactivity once and only if data actually changed
    if (updated) {
      runningJobs.value = jobsFiltered
    }
  }

  // --- ACTIONS ---

  async function fetchOverview() {
    isOverviewLoading.value = true
    try {
      const response = await ingestJobsService.getOverview()
      overview.value = response.data
    } catch (error) {
      snackbarStore.error('Failed to load ingest job counts', error)
    } finally {
      isOverviewLoading.value = false
    }
  }

  async function fetchRunning() {
    isRunningLoading.value = true
    try {
      const response = await ingestJobsService.getRunning()
      processIncomingJobs(response.data as DisplayJob[])
    } catch (error) {
      snackbarStore.error('Failed to load active ingest processes', error)
    } finally {
      isRunningLoading.value = false
    }
  }

  async function fetchUserJobs(showLoading = true) {
    if (showLoading) {
      isJobsLoading.value = true
    }
    try {
      const statusParam = selectedTab.value === 'processing' ? 'running' : selectedTab.value
      const response = await ingestJobsService.getUserJobs({
        page: page.value,
        limit: itemsPerPage.value,
        status: statusParam,
        search: searchQuery.value,
      })
      userJobs.value = response.data.data
      totalJobsCount.value = response.data.total
    } catch (error) {
      snackbarStore.error('Failed to load ingest queue details', error)
    } finally {
      isJobsLoading.value = false
    }
  }

  async function triggerScan() {
    try {
      await ingestJobsService.scan()
      snackbarStore.success('Library folder scan triggered successfully')
    } catch (error) {
      snackbarStore.error('Failed to trigger folder scan', error)
      throw error
    }
  }

  async function retryJob(jobId: number) {
    try {
      await ingestJobsService.retry(jobId)
      snackbarStore.success(`Ingest Job #${jobId} scheduled for retry`)
      await Promise.all([fetchOverview(), fetchUserJobs(true)])
    } catch (error) {
      snackbarStore.error(`Failed to retry Ingest Job #${jobId}`, error)
      throw error
    }
  }

  async function pollTick() {
    const promises: Promise<void>[] = [fetchOverview(), fetchRunning()]
    if (needsDetailsScope.value > 0) {
      promises.push(fetchUserJobs(false))
    }
    await Promise.all(promises)
  }

  function startPolling(includeDetails = false) {
    activeSubscribers.value++
    if (includeDetails) {
      needsDetailsScope.value++
    }

    if (activeSubscribers.value === 1) {
      lastTickTime = Date.now()

      pollTick()
      pollingIntervalId = setInterval(() => {
        pollTick()
      }, 3000)

      if (!trickleIntervalId) {
        // Reduced frequency from 50ms to 100ms for significant idle performance gain
        trickleIntervalId = setInterval(() => {
          tickTrickleAndCleanup()
        }, 100)
      }
    } else if (includeDetails && needsDetailsScope.value === 1) {
      fetchUserJobs(true)
    }
  }

  function stopPolling(includeDetails = false) {
    activeSubscribers.value = Math.max(0, activeSubscribers.value - 1)
    if (includeDetails) {
      needsDetailsScope.value = Math.max(0, needsDetailsScope.value - 1)
    }

    if (activeSubscribers.value === 0) {
      if (pollingIntervalId) {
        clearInterval(pollingIntervalId)
        pollingIntervalId = null
      }
      if (trickleIntervalId) {
        clearInterval(trickleIntervalId)
        trickleIntervalId = null
      }
    }
  }

  return {
    overview,
    runningJobs,
    isOverviewLoading,
    isRunningLoading,
    userJobs,
    totalJobsCount,
    isJobsLoading,
    page,
    itemsPerPage,
    searchQuery,
    selectedTab,
    fetchUserJobs,
    triggerScan,
    retryJob,
    pollTick,
    startPolling,
    stopPolling,
  }
})
