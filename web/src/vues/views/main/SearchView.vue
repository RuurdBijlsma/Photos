<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useRoute } from 'vue-router'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import SearchFilterMenu from '@/vues/components/ui/SearchFilterMenu.vue'
import searchService from '@/scripts/services/searchService.ts'
import { MONTHS } from '@/scripts/constants.ts'
import { useSearchStore } from '@/scripts/stores/searchStore.ts'

const snackStore = useSnackbarsStore()
const searchStore = useSearchStore()
const route = useRoute()

const results = ref<SimpleTimelineItem[]>([])
const loading = ref(false)
const showLoadingUI = ref(false)
let loadingTimer: ReturnType<typeof setTimeout> | null = null
const searchCache = new Map<string, SimpleTimelineItem[]>()
const SEARCH_LIMIT = 500
const offset = ref(0)
const hasMore = ref(true)
const loadingMore = ref(false)

let currentSearchId = 0
let activeAbortController: AbortController | null = null

// URL Source of Truth Computations
const query = computed(() => (route.query.query as string) || '')

const hasFilters = computed(() => {
  return (
    !!route.query.countries ||
    !!route.query.people ||
    !!route.query.exclude ||
    !!route.query.start ||
    !!route.query.end ||
    (route.query.type !== undefined && route.query.type !== 'all')
  )
})

const isFilterOnlyBrowse = computed(
  () => !query.value && hasFilters.value && !searchStore.searchImage,
)
const isEmptySearch = computed(() => !query.value && !hasFilters.value && !searchStore.searchImage)

// Date Range ISO Helper
function urlParamToISO(param: string | undefined, endOfMonth = false): string | undefined {
  if (!param) return undefined

  // Check if it's already YYYY-MM-DD day granularity
  if (/^\d{4}-\d{2}-\d{2}/.test(param)) {
    const date = new Date(param + 'T00:00:00Z')
    if (isNaN(date.getTime())) return undefined
    if (endOfMonth) {
      date.setUTCHours(23, 59, 59, 999)
    }
    return date.toISOString()
  }

  const monthStr = param.substring(0, 3).toLowerCase()
  const year = parseInt(param.substring(3))
  const monthIndex = MONTHS.findIndex((m) => m.toLowerCase().startsWith(monthStr))
  if (monthIndex === -1) return undefined
  const date = new Date(Date.UTC(year, monthIndex, 1))
  if (endOfMonth) {
    date.setUTCMonth(date.getUTCMonth() + 1)
    date.setUTCMilliseconds(-1)
  }
  return date.toISOString()
}

function getSearchParams(isLoadMore: boolean) {
  if (isLoadMore) {
    if (loadingMore.value || !hasMore.value) return
    loadingMore.value = true
  } else {
    // Removed the guard preventing execution when already loading to allow updates to override
    offset.value = 0
    hasMore.value = true
    if (loadingTimer) clearTimeout(loadingTimer)
    loading.value = true
    if (results.value.length === 0) showLoadingUI.value = true
    else loadingTimer = setTimeout(() => (showLoadingUI.value = true), 300)
  }
  return {
    query: query.value,
    limit: SEARCH_LIMIT,
    offset: offset.value,
    startDate: urlParamToISO(route.query.start as string),
    endDate: urlParamToISO(route.query.end as string, true),
    countryCodes: (route.query.countries as string) || '',
    mediaType: (route.query.type as 'all' | 'photo' | 'video' | 'panorama') || 'all',
    negativeQuery: (route.query.exclude as string) || undefined,
    personIds: (route.query.people as string) || '',
    allFacesRequired: route.query.peopleAnd === '1' ? true : undefined,
    sortBy: (isFilterOnlyBrowse.value
      ? 'date'
      : (route.query.sort as 'date' | 'relevancy') || 'relevancy') as 'date' | 'relevancy',
  }
}

async function executeSearch(isLoadMore = false) {
  // If we are not performing an image search, verify that we actually have a text query or active filters.
  if (!searchStore.searchImage && !query.value && !hasFilters.value) {
    results.value = []
    return
  }

  const searchId = ++currentSearchId

  if (!isLoadMore) {
    // Abort any ongoing query
    if (activeAbortController) {
      activeAbortController.abort()
    }
    activeAbortController = new AbortController()
  }

  const searchParams = getSearchParams(isLoadMore)
  if (!searchParams) return

  const signal = !isLoadMore ? activeAbortController?.signal : undefined

  try {
    let items: SimpleTimelineItem[] = []

    if (searchStore.searchImage) {
      // Execute Image Search
      const imageFile = searchStore.searchImage
      const response = searchStore.searchImageSessionId
        ? await searchService.searchByImageSession(
            searchStore.searchImageSessionId,
            searchParams,
            signal,
          )
        : await searchService.searchByImage(imageFile, searchParams, signal)
      if (!searchStore.searchImageSessionId && searchStore.searchImage === imageFile) {
        searchStore.searchImageSessionId = response.sessionId ?? null
      }
      items = response.items
      console.log('[IMAGE] SearchPage results', response)
    } else {
      // Execute Text/Filter Search with Cache Check
      const key = JSON.stringify(searchParams)
      if (searchCache.has(key)) {
        items = searchCache.get(key)!
      } else {
        const response = await searchService.search(searchParams, signal)
        items = response.items
        console.log('SearchPage results', items)
        requestIdleCallback(() => searchCache.set(key, items))
      }
    }

    // Only apply modifications if this is still the active search
    if (searchId !== currentSearchId) {
      return
    }

    if (isLoadMore) {
      results.value = [...results.value, ...items]
    } else {
      results.value = items
    }

    hasMore.value = items.length === SEARCH_LIMIT
    offset.value += items.length
  } catch (e) {
    if (searchId === currentSearchId) {
      snackStore.error('Could not perform search', e)
    }
  } finally {
    if (searchId === currentSearchId) {
      if (loadingTimer) clearTimeout(loadingTimer)
      loading.value = false
      loadingMore.value = false
      showLoadingUI.value = false
      if (!isLoadMore) {
        activeAbortController = null
      }
    }
  }
}

onUnmounted(() => {
  if (loadingTimer) clearTimeout(loadingTimer)
  if (activeAbortController) activeAbortController.abort()
})

watch([() => route.query, () => searchStore.searchImage], () => executeSearch(false), {
  immediate: true,
})
</script>

<template>
  <simple-timeline
    :timeline-items="showLoadingUI ? [] : results"
    view-link="/search/view/"
    :loading-more="loadingMore"
    @load-more="executeSearch"
  >
    <div class="search-options">
      <h2 class="search-query-title">
        <v-icon class="mr-5 search-query-icon" icon="mdi-magnify" />
        <template v-if="query">
          Search for “<span class="search-query-highlight">{{ query }}</span
          >”
        </template>
        <template v-else-if="searchStore.searchImage">
          Images similar to
          <img
            class="image-preview"
            v-if="searchStore.imagePreview"
            :src="searchStore.imagePreview"
            height="50"
            width="auto"
            alt="Search image"
          />
        </template>
        <template v-else-if="hasFilters">Filtered results</template>
        <template v-else>Search</template>
      </h2>
      <v-spacer />
      <search-filter-menu />
    </div>
    <div class="search-margin"></div>
    <div v-if="isEmptySearch && !showLoadingUI && results.length === 0" class="search-empty-state">
      <v-icon icon="mdi-magnify" size="100" class="mb-4 search-empty-icon" />
      <h2>Start a search</h2>
      <p>Enter a term in the search bar above, or use filters to browse your library.</p>
    </div>
    <div class="loading-indicator" v-if="showLoadingUI">
      <h2 class="search-summary">
        Searching for "<span class="query-span">{{ query }}</span
        >"...
      </h2>
      <v-progress-circular class="mt-6" :size="70" indeterminate />
    </div>
  </simple-timeline>
</template>

<style scoped>
.search-summary {
  font-weight: 400;
}

.query-span {
  font-style: italic;
  color: rgb(var(--v-theme-on-surface-bright));
  font-weight: 600;
}

.search-options {
  padding: 15px 20px;
  padding-bottom: 13px;
  border-radius: 10px;
  display: flex;
  align-items: center;
}

@media screen and (max-width: 1350px) {
  .search-query-title {
    display: none;
  }
}

.search-query-title {
  font-weight: 400;
  white-space: nowrap;
  padding-right: 15px;
  margin: 0;
  display: flex;
  align-items: center;
}

.image-preview {
  margin-top: -10px;
  margin-bottom: -10px;
  margin-left: 20px;
  border-radius: 7px;
  border: 2px solid rgba(var(--v-theme-on-surface), 0.25) !important;
}

.search-query-icon {
  opacity: 0.5;
  font-size: 30px;
}

.search-query-highlight {
  color: rgb(var(--v-theme-primary));
  font-weight: 600;
}

.search-margin {
  height: 10px;
}

.loading-indicator {
  padding: 20px 40px 30px;
  text-align: center;
}

.search-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 24px 120px;
  text-align: center;
}

.search-empty-state h2 {
  margin: 0;
  font-weight: 500;
  opacity: 0.85;
}

.search-empty-state p {
  margin: 12px 0 0;
  max-width: 360px;
  opacity: 0.6;
}

.search-empty-icon {
  opacity: 0.2;
}
</style>
