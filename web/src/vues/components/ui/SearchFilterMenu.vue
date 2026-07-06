<script setup lang="ts">
import { computed, nextTick, ref, watch, onMounted } from 'vue'
import HistogramDateRangePicker from '@/vues/components/ui/HistogramDateRangePicker.vue'
import { useRoute, useRouter } from 'vue-router'
import { useObjStorage } from '@/scripts/utils.ts'
import { useDebounceFn } from '@vueuse/core'
import { MONTHS } from '@/scripts/constants.ts'
import searchService from '@/scripts/services/searchService.ts'
import peopleService from '@/scripts/services/peopleService.ts'
import type { SearchFilterRanges } from '@/scripts/types/api/search.ts'

const route = useRoute()
const router = useRouter()

const filterRanges = useObjStorage<SearchFilterRanges | null>('searchFilterRanges', null)
const showFilters = ref(false)

// URL Source of Truth
const query = computed(() => (route.query.query as string) || '')
const filterMediaType = computed({
  get: () => (route.query.type as 'all' | 'photo' | 'video' | 'panorama') || 'all',
  set: (val) => updateURL({ type: val === 'all' ? undefined : val }),
})
const filterPeople = computed({
  get: () => (route.query.people ? (route.query.people as string).split(',') : []),
  set: (val) =>
    updateURL({
      people: val?.length ? val.join(',') : undefined,
      peopleAnd: val && val.length >= 2 ? (route.query.peopleAnd as string) : undefined,
    }),
})
const filterPeopleMatchAll = computed({
  get: () => route.query.peopleAnd === '1',
  set: (val) => updateURL({ peopleAnd: val ? '1' : undefined }),
})

function formatPeopleFilterLabel(names: string[], matchAll: boolean) {
  if (names.length === 0) return ''
  if (names.length === 1) return names[0]!
  const joiner = matchAll ? ' and ' : ' or '
  const last = names[names.length - 1]!
  return `${names.slice(0, -1).join(', ')}${joiner}${last}`
}

function getNamesFromIds(ids: string[]) {
  if (!filterRanges.value) return ids
  return ids.map((id) => {
    const p = filterRanges.value?.people.find((p) => p[1] === id)
    return p ? p[0] : id
  })
}

const filterNegativeQuery = computed({
  get: () => (route.query.exclude as string) || null,
  set: (val) => updateURL({ exclude: val || undefined }),
})

const filterCountries = computed({
  get: () => (route.query.countries ? (route.query.countries as string).split(',') : []),
  set: (val) => updateURL({ countries: val?.length ? val.join(',') : undefined }),
})

const defaultSortDirection: 'date' | 'relevancy' = 'relevancy'
const sortDirection = computed({
  get: () => (route.query.sort as 'date' | 'relevancy') || defaultSortDirection,
  set: (val) => updateURL({ sort: val === defaultSortDirection ? undefined : val }),
})

// Date Range Helpers
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

function dateToUrlParam(
  date: Date | null | undefined,
  granularity: 'month' | 'day',
): string | undefined {
  if (!date) return undefined
  if (granularity === 'day') {
    const y = date.getUTCFullYear()
    const m = String(date.getUTCMonth() + 1).padStart(2, '0')
    const d = String(date.getUTCDate()).padStart(2, '0')
    return `${y}-${m}-${d}`
  } else {
    return MONTHS[date.getUTCMonth()]!.substring(0, 3).toLowerCase() + date.getUTCFullYear()
  }
}

function formatDateShort(dateStr: string | undefined, granularity: 'month' | 'day') {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  if (granularity === 'day') {
    return date.toLocaleDateString(undefined, {
      day: 'numeric',
      month: 'short',
      year: 'numeric',
      timeZone: 'UTC',
    })
  }
  return date.toLocaleDateString(undefined, { month: 'short', year: 'numeric', timeZone: 'UTC' })
}

interface DateRange {
  startDate: Date | null
  endDate: Date | null
  active: boolean
  startGranularity: 'month' | 'day'
  endGranularity: 'month' | 'day'
}

const dateFilter = ref<DateRange>({
  startDate: null,
  endDate: null,
  active: false,
  startGranularity: 'month',
  endGranularity: 'month',
})

let ignoreDateFilterWatch = false

watch(
  [() => route.query.start, () => route.query.end],
  () => {
    if (ignoreDateFilterWatch) return

    const startIso = route.query.start ? urlParamToISO(route.query.start as string) : null
    const endIso = route.query.end ? urlParamToISO(route.query.end as string, true) : null

    const startDate = startIso ? new Date(startIso) : null
    const endDate = endIso ? new Date(endIso) : null

    const startGranularity =
      route.query.start && /^\d{4}-\d{2}-\d{2}/.test(route.query.start as string) ? 'day' : 'month'
    const endGranularity =
      route.query.end && /^\d{4}-\d{2}-\d{2}/.test(route.query.end as string) ? 'day' : 'month'

    dateFilter.value = {
      startDate,
      endDate,
      active: !!(startDate || endDate),
      startGranularity,
      endGranularity,
    }
  },
  { immediate: true },
)

watch(
  dateFilter,
  (newVal) => {
    const startParam =
      newVal.active && newVal.startDate
        ? dateToUrlParam(newVal.startDate, newVal.startGranularity)
        : undefined
    const endParam =
      newVal.active && newVal.endDate
        ? dateToUrlParam(newVal.endDate, newVal.endGranularity)
        : undefined

    if (startParam !== route.query.start || endParam !== route.query.end) {
      ignoreDateFilterWatch = true
      updateURL({
        start: startParam,
        end: endParam,
      })
      nextTick(() => {
        ignoreDateFilterWatch = false
      })
    }
  },
  { deep: true },
)

const hasFilters = computed(() => {
  return (
    filterCountries.value.length > 0 ||
    filterPeople.value.length > 0 ||
    filterNegativeQuery.value ||
    route.query.start !== undefined ||
    route.query.end !== undefined ||
    filterMediaType.value !== 'all'
  )
})

const isFilterOnlyBrowse = computed(() => !query.value && hasFilters.value)
const isEmptySearch = computed(() => !query.value && !hasFilters.value)

const debouncedPush = useDebounceFn((query: Record<string, string>) => {
  router.push({ query })
}, 100)

const updateURL = (newParams: Record<string, string | undefined>) => {
  const currentQuery = { ...route.query }
  const nextQuery = { ...currentQuery, ...newParams }

  // Clean up undefined/null/empty/default values
  Object.keys(nextQuery).forEach((key) => {
    if (
      nextQuery[key] === undefined ||
      nextQuery[key] === null ||
      nextQuery[key] === '' ||
      (key === 'type' && nextQuery[key] === 'all') ||
      (key === 'sort' && nextQuery[key] === defaultSortDirection)
    ) {
      delete nextQuery[key]
    }
  })

  // Only push if something actually changed
  if (JSON.stringify(currentQuery) !== JSON.stringify(nextQuery)) {
    debouncedPush(nextQuery as Record<string, string>)
  }
}

function clearFilters() {
  router.push({ query: { query: query.value } })
}

async function fetchFilterRanges() {
  try {
    const { data } = await searchService.filterRanges()
    filterRanges.value = data
  } catch (e) {
    console.warn("Couldn't fetch filter ranges", e)
  }
}

onMounted(() => {
  fetchFilterRanges()
})

const activeFilterChips = computed(() => {
  const chips: {
    id: string
    type: string
    label: string
    clear: () => void
    tooltip?: string
    countries?: { code: string; name: string }[]
  }[] = []

  // Date Range
  if (route.query.start || route.query.end) {
    const startGran =
      route.query.start && /^\d{4}-\d{2}-\d{2}/.test(route.query.start as string) ? 'day' : 'month'
    const endGran =
      route.query.end && /^\d{4}-\d{2}-\d{2}/.test(route.query.end as string) ? 'day' : 'month'

    const start = route.query.start
      ? formatDateShort(urlParamToISO(route.query.start as string), startGran)
      : null
    const end = route.query.end
      ? formatDateShort(urlParamToISO(route.query.end as string, true), endGran)
      : null
    let label = ''
    if (start && end) label = `Date: ${start} - ${end}`
    else if (start) label = `From ${start}`
    else if (end) label = `Until ${end}`

    chips.push({
      id: 'date',
      type: 'Date range',
      label,
      clear: () => {
        updateURL({ start: undefined, end: undefined })
      },
    })
  }

  // Media Type
  if (filterMediaType.value !== 'all') {
    let prettyName = 'Photos'
    if (filterMediaType.value === 'video') prettyName = 'Videos'
    else if (filterMediaType.value === 'panorama') prettyName = '360 Photos'

    chips.push({
      id: 'mediaType',
      type: 'Media type',
      label: prettyName,
      clear: () => (filterMediaType.value = 'all'),
    })
  }

  // People
  if (filterPeople.value.length > 0) {
    chips.push({
      id: 'people',
      type: 'People',
      label: formatPeopleFilterLabel(
        getNamesFromIds(filterPeople.value),
        filterPeopleMatchAll.value,
      ),
      clear: () => updateURL({ people: undefined, peopleAnd: undefined }),
    })
  }

  // Countries
  if (filterCountries.value.length > 0 && filterRanges.value) {
    const countries = filterCountries.value.map((code) => {
      const country = filterRanges.value?.countries.find((c) => c[0] === code)
      return { code, name: (country ? country[1] : null) ?? code }
    })

    chips.push({
      id: 'countries',
      type: 'Countries',
      tooltip: countries
        .map((c) => c.name)
        .join(', ')
        .replace(/, ([^,]*)$/, ' or $1'),
      label: '',
      countries: countries,
      clear: () => (filterCountries.value = []),
    })
  }

  // Exclude
  if (filterNegativeQuery.value) {
    chips.push({
      id: 'exclude',
      type: 'Exclude',
      label: `Exclude: "${filterNegativeQuery.value}"`,
      clear: () => (filterNegativeQuery.value = null),
    })
  }

  return chips
})
</script>

<template>
  <div class="advanced-search-container d-flex align-center">
    <div class="active-filters-chips mr-2">
      <v-chip
        v-for="chip in activeFilterChips"
        :key="chip.id"
        closable
        size="small"
        @click:close="chip.clear"
        variant="tonal"
        v-tooltip="{
          location: 'top',
          text: chip.tooltip || chip.type,
        }"
      >
        <template v-if="chip.countries" #prepend>
          <img
            v-for="(country, index) in chip.countries"
            :key="country.code"
            :src="`https://purecatamphetamine.github.io/country-flag-icons/3x2/${country.code}.svg`"
            width="20"
            height="15"
            style="object-fit: cover; border-radius: 2px"
            :style="{
              marginRight: index === chip.countries.length - 1 && !chip.label ? '0' : '4px',
            }"
            :alt="'Flag of ' + country.name"
          />
        </template>
        {{ chip.label }}
      </v-chip>
    </div>

    <v-menu
      location="bottom center"
      v-model="showFilters"
      :close-on-content-click="false"
      content-class="search-filter-menu"
    >
      <template v-slot:activator="{ props }">
        <v-btn v-bind="props" variant="text" rounded prepend-icon="mdi-tune-variant">
          Filters
        </v-btn>
      </template>

      <v-card
        :min-width="350"
        variant="flat"
        rounded="xl"
        class="search-filters"
        :loading="filterRanges === null"
      >
        <v-card-text>
          <div class="date-range-filter px-4 py-2">
            <HistogramDateRangePicker v-model="dateFilter" />
          </div>

          <v-divider class="mt-4 ml-3 mr-3 mb-4" />

          <div class="small-filters">
            <div class="media-type">
              <p class="mt-2 mb-2 font-weight-medium">Media type</p>
              <v-chip-group mandatory v-model="filterMediaType" color="primary" variant="text">
                <v-chip value="all">All</v-chip>
                <v-chip value="photo">Photos</v-chip>
                <v-chip value="video">Videos</v-chip>
                <v-chip value="panorama">360 Photos</v-chip>
              </v-chip-group>
            </div>

            <div class="person-name" v-if="filterRanges && filterRanges.people.length > 0">
              <p class="mt-2 mb-2 font-weight-medium">People</p>
              <v-select
                width="430"
                rounded
                hide-details
                clearable
                placeholder="Anyone"
                item-title="name"
                item-value="personId"
                variant="outlined"
                bg-color="surface-container-lowest"
                base-color="primary"
                density="comfortable"
                multiple
                chips
                closable-chips
                v-model="filterPeople"
                :items="filterRanges.people.map((p) => ({ name: p[0], personId: p[1] }))"
              >
                <template #item="{ props, item }">
                  <v-list-item v-bind="props" :title="undefined">
                    <template #prepend>
                      <v-avatar>
                        <v-img
                          v-if="item.name"
                          :src="peopleService.getPersonThumbnail(item.personId)"
                          style="object-fit: cover"
                          :alt="item.name"
                        />
                      </v-avatar>
                    </template>
                    <v-list-item-title class="ml-3">
                      {{ item.name }}
                    </v-list-item-title>
                  </v-list-item>
                </template>
              </v-select>
              <v-checkbox
                v-if="filterPeople.length >= 2"
                v-model="filterPeopleMatchAll"
                hide-details
                density="compact"
                color="primary"
                label="Must include all selected people"
                class="mt-1"
              />
            </div>

            <div class="country-code" v-if="filterRanges">
              <p class="mt-2 mb-2 font-weight-medium">Country</p>
              <v-select
                v-model="filterCountries"
                :items="
                  filterRanges.countries.map((c) => ({
                    code: c[0],
                    name: c[1],
                  }))
                "
                item-title="name"
                item-value="code"
                variant="outlined"
                bg-color="surface-container-lowest"
                base-color="primary"
                density="comfortable"
                width="430"
                rounded
                hide-details
                clearable
                multiple
                chips
                closable-chips
                placeholder="Any country"
              >
                <template #item="{ props, item }">
                  <v-list-item v-bind="props" :title="undefined">
                    <template #prepend>
                      <img
                        v-if="item.code"
                        :src="`https://purecatamphetamine.github.io/country-flag-icons/3x2/${item.code}.svg`"
                        width="20"
                        height="15"
                        style="object-fit: cover"
                        :alt="'Flag of ' + item.name"
                      />
                    </template>
                    <v-list-item-title class="ml-3">
                      {{ item.name }}
                    </v-list-item-title>
                  </v-list-item>
                </template>
              </v-select>
            </div>

            <div class="negative-query">
              <p class="mt-2 mb-2 font-weight-medium">
                Exclude
                <v-icon
                  size="16"
                  icon="mdi-information-outline"
                  class="ml-2"
                  v-tooltip="{
                    location: 'top',
                    text: 'Remove results matching these terms (e.g. “cat” + exclude “orange” → non-orange cats)',
                  }"
                ></v-icon>
              </p>
              <v-text-field
                v-model="filterNegativeQuery"
                hide-details
                clearable
                placeholder="E.g. “orange”"
                variant="outlined"
                bg-color="surface-container-lowest"
                base-color="primary"
                density="comfortable"
                width="430"
                rounded
              />
            </div>
          </div>
        </v-card-text>
      </v-card>
    </v-menu>

    <v-btn
      icon="mdi-close"
      variant="plain"
      @click="clearFilters"
      class="clear-filters-button"
      density="comfortable"
      v-tooltip="{
        location: 'top',
        text: 'Clear filters',
      }"
      :style="{
        pointerEvents: hasFilters ? 'auto' : 'none',
        opacity: hasFilters ? '.6' : '0',
      }"
    ></v-btn>

    <!-- Sort Direction Button Group -->
    <v-btn
      v-if="isFilterOnlyBrowse"
      rounded="xl"
      color="primary"
      variant="tonal"
      class="sort-button-group ml-2"
      disabled
      v-tooltip="{
        location: 'top',
        text: 'Sorted by capture date',
      }"
    >
      <span>Date</span>
      <v-icon end icon="mdi-sort-calendar-ascending"></v-icon>
    </v-btn>

    <v-btn-toggle
      v-else-if="!isEmptySearch"
      v-tooltip="{
        location: 'top',
        text: 'Sort',
      }"
      v-model="sortDirection"
      divided
      rounded="xl"
      color="primary"
      variant="tonal"
      class="sort-button-group ml-2"
      mandatory
    >
      <v-btn value="date">
        <span>Date</span>
        <v-icon end icon="mdi-sort-calendar-ascending"></v-icon>
      </v-btn>

      <v-btn value="relevancy">
        <span>Relevancy</span>
        <v-icon end icon="mdi-sort-descending"></v-icon>
      </v-btn>
    </v-btn-toggle>
  </div>
</template>

<style scoped>
.advanced-search-container {
  display: flex;
  align-items: center;
}

.active-filters-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  max-height: 60px;
  align-items: center;
  overflow: hidden;
}

.search-filters {
  max-width: 1100px;
}

:deep(.search-filter-menu) .search-filters {
  border: 1px solid rgba(255, 255, 255, 0.1);
  background-color: rgba(var(--v-theme-surface-container-low), 0.95);
}

.small-filters {
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.small-filters > div {
  padding: 10px;
  padding-top: 0;
}

.clear-filters-button {
  transition: opacity 0.15s;
}

.date-range-text {
  align-items: center;
}

.sort-button-group {
  overflow-x: hidden;
}
</style>

<style>
.backdrop-blur .search-filter-menu .search-filters {
  background-color: rgba(var(--v-theme-surface-container-low), 0.8) !important;
  backdrop-filter: blur(12px) saturate(180%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(180%) !important;
}
</style>
