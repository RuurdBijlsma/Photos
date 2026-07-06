<script setup lang="ts">
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useResizeObserver } from '@vueuse/core'
import { useVirtualizer } from '@tanstack/vue-virtual'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import storageService from '@/scripts/services/storageService.ts'
import { prettyBytes } from '@/scripts/utils.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import type { StorageReviewItem } from '@/scripts/types/generated/timeline.ts'
import { useBinStore } from '@/scripts/stores/binStore.ts'
import { useViewPhotoStore } from '@/scripts/stores/timeline/viewPhotoStore.ts'
import StorageReviewCard from '@/vues/components/ui/StorageReviewCard.vue'

const route = useRoute()
const dialogStore = useDialogStore()
const snackbarStore = useSnackbarsStore()
const binStore = useBinStore()
const viewPhotoStore = useViewPhotoStore()

const items = ref<StorageReviewItem[]>([])
const loading = ref(false)
const actionLoading = ref(false)
const batchDownloading = ref(false)
const downloadingIds = ref<Set<string>>(new Set())
const selected = ref<Set<string>>(new Set())
const scrollContainer = useTemplateRef('scrollContainer')
const containerWidth = ref(0)

const ITEM_GAP = 14
const MIN_TILE_WIDTH = 240
const row_height = computed(() => (mode.value === 'blurry' ? 315 : 300))

const mode = computed<'review' | 'blurry'>(() =>
  route.path.includes('/blurry') ? 'blurry' : 'review',
)
const title = computed(() => (mode.value === 'blurry' ? 'Blurry photos' : 'Large photos & videos'))
const emptyIcon = computed(() =>
  mode.value === 'blurry' ? 'mdi-image-broken-variant' : 'mdi-harddisk',
)
const basePath = computed(() => (mode.value === 'blurry' ? '/storage/blurry' : '/storage/review'))
const totalSize = computed(() => items.value.reduce((sum, item) => sum + item.sizeBytes, 0))
const selectedSize = computed(() => {
  let sum = 0
  for (const item of items.value) {
    if (selected.value.has(item.id)) {
      sum += item.sizeBytes
    }
  }
  return sum
})

const allSelected = computed(
  () => items.value.length > 0 && selected.value.size === items.value.length,
)

const columns = computed(() => {
  if (containerWidth.value <= 0) return 1
  return Math.max(1, Math.floor((containerWidth.value + ITEM_GAP) / (MIN_TILE_WIDTH + ITEM_GAP)))
})
const tileWidth = computed(() => {
  const cols = columns.value
  return Math.floor((containerWidth.value - ITEM_GAP * (cols - 1)) / cols)
})
const rows = computed(() => {
  const result: StorageReviewItem[][] = []
  for (let i = 0; i < items.value.length; i += columns.value) {
    result.push(items.value.slice(i, i + columns.value))
  }
  return result
})

const virtualizerOptions = computed(() => ({
  count: rows.value.length,
  getScrollElement: () => scrollContainer.value,
  estimateSize: () => row_height.value + ITEM_GAP,
  overscan: 3,
}))
const rowVirtualizer = useVirtualizer(virtualizerOptions)

const virtualRows = computed(() => rowVirtualizer.value.getVirtualItems())
const virtualGridHeight = computed(() => rowVirtualizer.value.getTotalSize())

function toggleItem(id: string) {
  const next = new Set(selected.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selected.value = next
}

function toggleAll() {
  selected.value = allSelected.value ? new Set() : new Set(items.value.map((item) => item.id))
}

function setDownloading(id: string, value: boolean) {
  const next = new Set(downloadingIds.value)
  if (value) next.add(id)
  else next.delete(id)
  downloadingIds.value = next
}

async function loadItems() {
  loading.value = true
  selected.value = new Set()
  try {
    const response =
      mode.value === 'blurry'
        ? await storageService.getBlurryItems()
        : await storageService.getReviewItems()
    console.log('review items', response.items)
    items.value = response.items
  } catch (e) {
    snackbarStore.error('Could not load storage review items', e)
  } finally {
    loading.value = false
  }
}

async function downloadItem(item: StorageReviewItem) {
  if (downloadingIds.value.has(item.id)) return
  setDownloading(item.id, true)
  try {
    const response = await mediaItemService.downloadMediaFileById(item.id)
    let filename = item.filename
    const contentDisposition =
      response.headers?.['content-disposition'] || response.headers?.['Content-Disposition']
    if (contentDisposition) {
      const match = contentDisposition.match(/filename\*?=(?:UTF-8'')?['"]?([^;\r\n"']+)['"]?/i)
      if (match && match[1]) {
        filename = decodeURIComponent(match[1])
      }
    }
    const url = window.URL.createObjectURL(response.data)
    const link = document.createElement('a')
    link.href = url
    link.download = filename
    link.click()
    window.URL.revokeObjectURL(url)
    snackbarStore.enqueue({ message: 'Download started', icon: 'mdi-download-outline' })
  } catch (e) {
    snackbarStore.error('Could not download item', e)
  } finally {
    setDownloading(item.id, false)
  }
}

async function downloadSelected() {
  // Evaluated on demand instead of inside an active computed watcher
  const targetItems = items.value.filter((item) => selected.value.has(item.id))
  if (batchDownloading.value || targetItems.length === 0) return
  batchDownloading.value = true
  snackbarStore.enqueue({
    message: `Preparing ${targetItems.length} download${targetItems.length === 1 ? '' : 's'}`,
    icon: 'mdi-download-outline',
  })
  try {
    for (const item of targetItems) {
      await downloadItem(item)
    }
  } finally {
    batchDownloading.value = false
  }
}

async function deleteItems(ids: string[]) {
  if (ids.length === 0) return

  const confirmed = await dialogStore.confirm({
    title: 'Move items to bin?',
    color: 'error',
    icon: 'mdi-delete',
    description: `Are you sure you want to delete ${ids.length} item${ids.length === 1 ? '' : 's'}? They will end up in your Bin where you can delete them permanently.`,
    confirmText: 'Move to bin',
  })
  if (!confirmed) return

  actionLoading.value = true
  try {
    await binStore.softDeleteItems(ids)
  } finally {
    actionLoading.value = false
  }
}

watch(
  [items, () => mode.value],
  () => {
    viewPhotoStore.ids = items.value.map((i) => i.id)
    viewPhotoStore.viewLink =
      mode.value === 'blurry' ? '/storage/blurry/view/' : '/storage/review/view/'
  },
  { immediate: true },
)

useResizeObserver(scrollContainer, (entries) => {
  if (entries[0]) containerWidth.value = entries[0].contentRect.width
})

watch(mode, loadItems)
onMounted(loadItems)
</script>

<template>
  <main-layout-container>
    <div class="storage-review">
      <div ref="scrollContainer" class="scroll-container">
        <header class="review-header">
          <div>
            <div class="breadcrumbs">
              <router-link class="crumb-link" to="/storage">Manage storage</router-link>
              <v-icon icon="mdi-chevron-right" size="18" />
              <span>{{ title }}</span>
            </div>
            <h1>{{ title }}</h1>
            <p>
              {{ items.length }} item{{ items.length === 1 ? '' : 's' }} &middot;
              {{ prettyBytes(totalSize) }}
            </p>
          </div>

          <div class="header-actions">
            <v-btn
              variant="tonal"
              v-if="selected.size !== 0"
              color="error"
              prepend-icon="mdi-delete"
              rounded="xl"
              class="text-none delete-btn"
              :loading="actionLoading"
              :disabled="selected.size === 0 || batchDownloading"
              @click="deleteItems([...selected])"
            >
              Delete
            </v-btn>
            <v-btn
              variant="tonal"
              v-if="selected.size !== 0"
              color="primary"
              prepend-icon="mdi-download-outline"
              rounded="xl"
              class="text-none stable-btn"
              :loading="batchDownloading"
              :disabled="actionLoading || downloadingIds.size > 0"
              @click="downloadSelected"
            >
              Download
            </v-btn>
            <v-btn
              variant="tonal"
              color="primary"
              rounded="xl"
              :prepend-icon="
                allSelected ? 'mdi-close' : 'mdi-checkbox-multiple-marked-circle-outline'
              "
              class="text-none stable-btn"
              :disabled="items.length === 0"
              @click="toggleAll"
            >
              {{ allSelected ? 'Clear' : 'Select all' }}
            </v-btn>
            <div class="selected-size">
              <span>{{ selected.size }} selected</span>
              <strong>{{ prettyBytes(selectedSize) }}</strong>
            </div>
          </div>
        </header>

        <div v-if="loading" class="state">
          <v-progress-circular indeterminate color="primary" size="50" />
        </div>

        <div v-else-if="items.length === 0" class="state empty-state">
          <v-icon :icon="emptyIcon" size="100" class="mb-4 opacity-20" />
          <h2>Nothing to review</h2>
          <p>This section is clear for now.</p>
        </div>

        <div
          v-else
          class="virtual-grid"
          :style="{
            height: `${virtualGridHeight}px`,
          }"
        >
          <div
            v-for="virtualRow in virtualRows"
            :key="String(virtualRow.key)"
            class="virtual-row"
            :style="{
              transform: `translateY(${virtualRow.start}px)`,
              height: `${row_height}px`,
              gap: `${ITEM_GAP}px`,
            }"
          >
            <storage-review-card
              v-for="item in rows[virtualRow.index]"
              :key="item.id"
              :item="item"
              :base-path="basePath"
              :tile-width="tileWidth"
              :is-selected="selected.has(item.id)"
              :is-selecting="selected.size > 0"
              :is-downloading="downloadingIds.has(item.id)"
              :action-loading="actionLoading"
              :batch-downloading="batchDownloading"
              @toggle="toggleItem(item.id)"
              @download="downloadItem(item)"
              @delete="deleteItems([item.id])"
            />
          </div>
        </div>
      </div>
    </div>

    <teleport to="body">
      <router-view />
    </teleport>
  </main-layout-container>
</template>

<style scoped>
.storage-review {
  height: 100%;
  overflow: hidden;
}

.scroll-container {
  height: 100%;
  overflow-y: auto;
  padding: 28px;
  scrollbar-width: none;
}

.scroll-container::-webkit-scrollbar {
  display: none;
}

.review-header {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  align-items: flex-start;
  margin-bottom: 24px;
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: 4px;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.9rem;
  margin-bottom: 8px;
}

.crumb-link {
  color: rgb(var(--v-theme-primary));
  text-decoration: none;
}

.crumb-link:hover {
  text-decoration: underline;
}

h1 {
  margin: 0;
  font-size: 2.4rem;
  font-weight: 600;
}

.review-header p {
  margin: 4px 0 0;
  color: rgb(var(--v-theme-on-surface-variant));
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.stable-btn {
  width: 150px;
}

.delete-btn {
  width: 120px;
}

.selected-size {
  min-width: 112px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  line-height: 1.2;
}

.selected-size span {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.78rem;
}

.selected-size strong {
  font-weight: 600;
}

.virtual-grid {
  position: relative;
  width: 100%;
}

.virtual-row {
  position: absolute;
  top: 0;
  left: 0;
  display: flex;
  width: 100%;
  contain: layout paint;
}

.state {
  min-height: 360px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-state {
  flex-direction: column;
  text-align: center;
}

.empty-state p {
  color: rgb(var(--v-theme-on-surface-variant));
}

@media (max-width: 760px) {
  .scroll-container {
    padding: 18px;
  }

  .review-header {
    flex-direction: column;
  }

  .header-actions {
    justify-content: flex-start;
  }

  .selected-size {
    align-items: flex-start;
  }
}
</style>
