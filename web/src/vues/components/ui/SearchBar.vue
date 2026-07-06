<script setup lang="ts">
import { ref, watch, useTemplateRef, onMounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useDebounceFn } from '@vueuse/core'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { type SimpleTimelineItem, SuggestionType } from '@/scripts/types/generated/timeline.ts'
import GridItem from '@/vues/components/timeline/timeline-components/GridItem.vue'
import searchService from '@/scripts/services/searchService.ts'
import { isLikelyJwt } from '@/scripts/utils.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import peopleService from '@/scripts/services/peopleService.ts'
import { useSearchStore } from '@/scripts/stores/searchStore.ts'

const router = useRouter()
const route = useRoute()
const snackStore = useSnackbarsStore()
const authStore = useAuthStore()
const searchStore = useSearchStore()
const searchInputEl = useTemplateRef('searchInput')
const searchContainer = useTemplateRef('searchContainer')

const query = ref('')
const results = ref<SimpleTimelineItem[]>([])

type SearchBarSuggestionType = SuggestionType | 'HISTORY'

interface SearchBarSuggestion {
  text: string
  suggestionType: SearchBarSuggestionType
  id?: string
}

const suggestions = ref<SearchBarSuggestion[]>([])
const loading = ref(false)
const isFocused = ref(false)
const selectedSuggestionIndex = ref(-1)

const SUGGESTION_PLACEHOLDER_KEY = 'search-suggestion-placeholder'
const MAX_HISTORY_SUGGESTIONS = 3
const MAX_TOTAL_SUGGESTIONS = 10
const HISTORY_STORAGE_KEY = 'search-history'
const MAX_HISTORY_SIZE = 200

const placeholder = ref<string | null>(
  localStorage.getItem(SUGGESTION_PLACEHOLDER_KEY) === null
    ? null
    : JSON.parse(localStorage.getItem(SUGGESTION_PLACEHOLDER_KEY)!),
)
const placeholderValue = computed(() => {
  if (searchStore.searchImage) return undefined
  return placeholder.value === null ? 'Search...' : `Search “${placeholder.value}”`
})
const searchHistory = ref<string[]>([])

function loadHistory() {
  const stored = localStorage.getItem(HISTORY_STORAGE_KEY)
  if (stored) {
    try {
      searchHistory.value = JSON.parse(stored)
    } catch (e) {
      console.error('Failed to parse search history', e)
      searchHistory.value = []
    }
  }
}

function saveToHistory(text: string) {
  const trimmed = text.trim()
  if (!trimmed) return
  // Remove if already exists to move it to the front
  searchHistory.value = searchHistory.value.filter((h) => h !== trimmed)
  searchHistory.value.unshift(trimmed)
  // Limit total history size
  if (searchHistory.value.length > MAX_HISTORY_SIZE) {
    searchHistory.value = searchHistory.value.slice(0, MAX_HISTORY_SIZE)
  }
  localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(searchHistory.value))
}

let latestRequestId = 0

async function performSearch(searchQuery: string | null) {
  if (!searchQuery?.trim()) {
    results.value = []
    return
  }

  const requestId = ++latestRequestId
  loading.value = true

  try {
    const { items } = await searchService.search({
      query: searchQuery,
      limit: MAX_TOTAL_SUGGESTIONS,
    })
    if (requestId === latestRequestId) {
      console.log('Search Results:', items)
      results.value = items
    }
  } catch (e) {
    if (requestId === latestRequestId) {
      snackStore.error('Search failed', e)
    }
  } finally {
    if (requestId === latestRequestId) {
      loading.value = false
    }
  }
}

async function fetchSuggestions(searchQuery: string | null) {
  if (!searchQuery?.trim()) {
    suggestions.value = searchHistory.value.slice(0, MAX_HISTORY_SUGGESTIONS * 2).map((text) => ({
      text,
      suggestionType: 'HISTORY',
    }))
    return
  }

  const q = searchQuery.toLowerCase()
  const historicMatches: SearchBarSuggestion[] = searchHistory.value
    .filter((h) => h.toLowerCase().includes(q))
    .slice(0, MAX_HISTORY_SUGGESTIONS)
    .map((text) => ({
      text,
      suggestionType: 'HISTORY',
    }))

  try {
    const { suggestions: fetchedSuggestions } = await searchService.suggestions(
      searchQuery,
      MAX_TOTAL_SUGGESTIONS,
    )
    console.log('fetchedSuggestions', fetchedSuggestions)
    const apiSuggestions: SearchBarSuggestion[] = fetchedSuggestions
      .filter((s) => {
        if (s.suggestionType === SuggestionType.ALBUM) return true
        if (s.suggestionType === SuggestionType.PERSON) return true
        return !historicMatches.some((h) => h.text.toLowerCase() === s.text.toLowerCase())
      })
      .map((s) => ({
        text: s.text,
        suggestionType: s.suggestionType as SearchBarSuggestionType,
        id: s.id,
      }))
    suggestions.value = [...historicMatches, ...apiSuggestions].slice(0, MAX_TOTAL_SUGGESTIONS)
  } catch (e) {
    console.error('Failed to fetch suggestions', e)
    suggestions.value = historicMatches
  }
}

function highlightMatch(text: string, match: string | null) {
  if (!match || !text) return text
  const escapedMatch = match.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const regex = new RegExp(`(${escapedMatch})`, 'gi')
  return text.replace(regex, '<strong>$1</strong>')
}

function selectSuggestion(suggestion: SearchBarSuggestion) {
  if (suggestion.id) {
    if (suggestion.suggestionType === SuggestionType.ALBUM) {
      router.push(`/album/${suggestion.id}`)
    } else if (suggestion.suggestionType === SuggestionType.PERSON) {
      router.push(`/person/${suggestion.id}`)
    }
    isFocused.value = false
    if (searchInputEl.value) {
      searchInputEl.value.blur()
    }
    return
  }
  query.value = suggestion.text
  suggestions.value = []
  isFocused.value = false
  if (searchInputEl.value) {
    searchInputEl.value.blur()
  }
  handleSubmit(false)
}

const debouncedFetchSuggestions = useDebounceFn((val: string | null) => {
  fetchSuggestions(val)
}, 16)

const debouncedPerformSearch = useDebounceFn((val: string | null) => {
  performSearch(val)
}, 100)

watch(query, (newVal) => {
  selectedSuggestionIndex.value = -1
  if (newVal && isLikelyJwt(newVal)) {
    const token = newVal.trim()
    query.value = ''
    if (searchInputEl.value) {
      searchInputEl.value.blur()
    }
    isFocused.value = false
    router.push(`/import-album/${token}`)
    return
  }
  debouncedFetchSuggestions(newVal)
  debouncedPerformSearch(newVal)
})

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (selectedSuggestionIndex.value < suggestions.value.length - 1) {
      selectedSuggestionIndex.value++
    }
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (selectedSuggestionIndex.value > -1) {
      selectedSuggestionIndex.value--
    }
  } else if (e.key === 'Enter') {
    if (
      selectedSuggestionIndex.value >= 0 &&
      selectedSuggestionIndex.value < suggestions.value.length
    ) {
      e.preventDefault()
      selectSuggestion(suggestions.value[selectedSuggestionIndex.value]!)
    }
  } else if (e.key === 'Escape') {
    isFocused.value = false
    if (searchInputEl.value) {
      searchInputEl.value.blur()
    }
  }
}

function handleDrop(e: DragEvent) {
  e.preventDefault()
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    const file = files[0]
    if (file.type.startsWith('image/')) {
      submitImage(file)
    }
  }
}

function handlePaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items
  if (items) {
    for (let i = 0; i < items.length; i++) {
      const item = items[i]
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile()
        if (file) {
          submitImage(file)
          e.preventDefault()
          break
        }
      }
    }
  }
}

function clearImageState() {
  searchStore.searchImage = null
  results.value = []
}

function clearImage(focus: boolean = true) {
  if (searchStore.searchImage) {
    clearImageState()
    if (focus) {
      isFocused.value = true
      setTimeout(() => {
        if (searchInputEl.value) {
          searchInputEl.value.focus()
        }
      }, 50)
    } else {
      isFocused.value = false
    }

    // Clean up route query parameter when removing the image
    const nextQuery = { ...route.query }
    delete nextQuery.mode
    router.push({
      path: '/search',
      query: nextQuery,
    })
  }
}

function submitImage(file: File) {
  searchStore.searchImage = file
  query.value = ''

  const nextQuery: Record<string, string> = {
    ...route.query,
    mode: 'image',
  }
  // Clear text-query parameter when doing image search
  delete nextQuery.query
  router.push({
    path: '/search',
    query: nextQuery,
  })
  return
}

function handleSubmit(isManual = true) {
  const trimmed = query.value?.trim() ?? ''
  if (isManual && trimmed) {
    saveToHistory(trimmed)
  }
  if (searchInputEl.value) {
    searchInputEl.value.blur()
  }
  const nextQuery = { ...route.query }
  if (trimmed) {
    nextQuery.query = trimmed
  } else {
    delete nextQuery.query
  }
  router.push({
    path: '/search',
    query: nextQuery,
  })
}

function handleFocus() {
  isFocused.value = true
  if (!query.value?.trim()) {
    fetchSuggestions(null)
  }
}

function handleFocusOut(event: FocusEvent) {
  if (event.relatedTarget && searchContainer.value?.contains(event.relatedTarget as Node)) {
    return
  }
  isFocused.value = false
}

async function loadNextPlaceholder() {
  if (!authStore.isAuthenticated) return
  try {
    const { data } = await searchService.randomSuggestion()
    if (data) localStorage[SUGGESTION_PLACEHOLDER_KEY] = JSON.stringify(data)
  } catch (err) {
    console.warn(`Couldn't fetch random suggestion for placeholder`, err)
  }
}

onMounted(() => {
  loadHistory()
  if (route.query.query) {
    query.value = route.query.query.toString()
  }
  requestIdleCallback(loadNextPlaceholder)
})

watch(
  () => [route.path, route.query.query],
  ([newPath, newQuery]) => {
    isFocused.value = false
    if (newPath && !newPath.toString().startsWith('/search')) {
      query.value = ''
      clearImageState()
    } else {
      query.value = newQuery ? newQuery.toString() : ''
    }
  },
)
</script>

<template>
  <div class="search-section">
    <div
      ref="searchContainer"
      class="search-centered-section"
      @focusout="handleFocusOut"
      @dragover.prevent
      @dragleave.prevent
      @drop.prevent="handleDrop"
    >
      <form @submit.prevent="() => handleSubmit(true)">
        <label
          class="search-bar"
          tabindex="-1"
          :class="{ 'is-focused': isFocused && !searchStore.searchImage }"
        >
          <span class="search-icon-div">
            <v-icon
              class="search-icon"
              :icon="
                loading
                  ? 'mdi-loading mdi-spin'
                  : searchStore.searchImage
                    ? 'mdi-image-search-outline'
                    : 'mdi-magnify'
              "
            ></v-icon>
          </span>
          <img
            v-if="searchStore.imagePreview"
            :src="searchStore.imagePreview"
            class="image-preview"
            alt="Search image"
          />
          <v-text-field
            ref="searchInput"
            v-model="query"
            class="search-text-field"
            :placeholder="placeholderValue"
            rounded
            autocomplete="off"
            hide-details
            clearable
            @focus="handleFocus"
            @click:clear="results = []"
            @keydown="handleKeyDown"
            @paste="handlePaste"
            :disabled="!!searchStore.searchImage"
          />
          <v-btn
            class="image-clear-button"
            icon="mdi-close"
            v-if="searchStore.searchImage"
            variant="plain"
            @click="clearImage"
            density="compact"
          ></v-btn>
        </label>
      </form>

      <div
        v-if="
          isFocused && !searchStore.searchImage && (query?.length > 0 || suggestions.length > 0)
        "
        class="search-suggestions"
        tabindex="-1"
      >
        <div class="search-suggestions-inner">
          <div v-if="suggestions.length > 0" class="suggestions-list">
            <div
              v-for="(suggestion, index) in suggestions"
              :key="index"
              class="suggestion-item"
              :class="{ 'suggestion-selected': index === selectedSuggestionIndex }"
              @click="selectSuggestion(suggestion)"
            >
              <v-icon
                v-if="suggestion.suggestionType === 'HISTORY'"
                icon="mdi-history"
                size="small"
                class="suggestion-icon"
              />
              <v-icon
                v-else-if="suggestion.suggestionType === SuggestionType.ALBUM"
                icon="mdi-image-album"
                size="small"
                class="suggestion-icon"
              />
              <v-avatar
                v-else-if="suggestion.suggestionType === SuggestionType.PERSON"
                size="small"
              >
                <v-img :src="peopleService.getPersonThumbnail(suggestion.id)"></v-img>
              </v-avatar>
              <span v-html="highlightMatch(suggestion.text, query)"></span>
            </div>
          </div>

          <div v-if="loading && results.length === 0 && suggestions.length === 0">Searching...</div>
          <div v-else-if="results.length === 0 && suggestions.length === 0">No results found.</div>
          <div v-if="results.length > 0" class="search-results-section">
            <div class="results-header">Media</div>
            <div class="search-results">
              <grid-item
                v-for="result in results"
                :key="result.id"
                :media-item="result"
                :width="120 * result.ratio"
                :height="120"
                :thumbnail-size="240"
                :is-scrolling-fast="false"
                class="search-grid-item"
                view-link="/search/view/"
                :query="{ query }"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-section {
  position: fixed;
  top: 7px;
  width: 100%;
}

.search-centered-section {
  width: calc(50% - 100px);
  max-width: 700px;
  margin: 0 auto;
  position: relative;
  flex-grow: 1;
}

.search-suggestions {
  color: rgb(var(--v-theme-on-surface-container-high));
  top: 25px;
  position: absolute;
  z-index: 5;
  width: 100%;
  padding-top: 25px;
  border-bottom-right-radius: 25px;
  border-bottom-left-radius: 25px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background-color: rgba(var(--v-theme-surface-container-high), 0.95);
  box-shadow: 0 10px 20px 0 rgba(0, 0, 0, 0.2);
}

.backdrop-blur .search-suggestions {
  background-color: rgba(var(--v-theme-surface-container-high), 0.8);
  backdrop-filter: saturate(250%) blur(12px) !important;
}

.suggestions-list {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.suggestion-item {
  padding: 8px 12px;
  cursor: pointer;
  border-radius: 10px;
  transition: background-color 0.2s;
  display: flex;
  align-items: center;
  gap: 10px;
}

.suggestion-item:hover,
.suggestion-item.suggestion-selected {
  background-color: rgba(var(--v-theme-on-surface), 0.1);
}

.suggestion-icon {
  opacity: 0.6;
}

.suggestion-item :deep(strong) {
  color: rgb(var(--v-theme-primary));
}

.search-results-section {
  margin-top: 10px;
}

.results-header {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 1px;
  opacity: 0.6;
  margin-bottom: 8px;
  padding-left: 5px;
}

.search-suggestions-inner {
  padding: 15px;
}

.search-bar {
  position: absolute;
  width: 100%;
  cursor: text;
  height: 50px;
  border-radius: 25px;
  background-color: rgba(var(--v-theme-on-background), 0.07);
  display: flex;
  flex-direction: row;
  z-index: 10;
}

.search-bar:focus {
  outline: none;
}

.search-bar.is-focused {
  background-color: white;
  color: black;
  box-shadow: 0 2px 2px 0 rgba(0, 0, 0, 0.2);
}

.search-icon-div {
  width: 40px;
  height: 50px;
  display: flex;
  justify-content: flex-end;
  align-items: center;
}

.image-preview {
  height: 40px;
  border-radius: 8px;
  object-fit: cover;
  max-width: 600px;
  margin: 5px;
  margin-left: 15px;
  border: 2px solid rgba(var(--v-theme-on-surface), 0.25) !important;

  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.35) !important;
}

.image-clear-button {
  margin: 12px;
}

.search-text-field {
  margin-top: -3px;
}

.search-bar:deep(.v-field__overlay) {
  display: none;
}

.search-bar:deep(.v-field__outline) {
  display: none;
}

.mdi-spin {
  animation: mdi-spin 2s infinite linear;
}
@keyframes mdi-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.search-results {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  padding-bottom: 5px;
}

.search-grid-item {
  border-radius: 20px;
}
</style>
