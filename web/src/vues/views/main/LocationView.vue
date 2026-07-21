<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import LocationMediaMap from '@/vues/components/map/LocationMediaMap.vue'
import { useRefreshFunction } from '@/scripts/composables/useRefreshFunction.ts'
import { getThumbnailHeight } from '@/scripts/utils.ts'

const route = useRoute()
const exploreStore = useExploreStore()
const simpleTimelineRef = useTemplateRef('simpleTimeline')

const isInitialLoad = ref(true)
const fetched = ref(false)

const locationId = computed(() => {
  const rawId = route.params.locationId
  const idStr = Array.isArray(rawId) ? rawId[0] : rawId
  return idStr || null
})

const details = computed(() => {
  if (locationId.value === null) return null
  return exploreStore.locations.get(locationId.value)?.location ?? null
})

const items = computed(() => {
  if (locationId.value === null) return []
  return exploreStore.locations.get(locationId.value)?.items ?? []
})

function photoCountText(count: number) {
  return `${count.toLocaleString()} item${count === 1 ? '' : 's'}`
}

const primaryName = computed(() => details.value?.name || '')
const secondaryContext = computed(() => {
  if (!details.value) return ''
  const parts = []
  if (details.value.admin1 && details.value.admin1 !== details.value.name) {
    parts.push(details.value.admin1)
  }
  // Deduplicate secondary rendering when primary details already represent the country itself
  if (details.value.countryName && details.value.countryName !== details.value.name) {
    parts.push(details.value.countryName)
  }
  return parts.join(', ')
})

watch(
  locationId,
  () => {
    isInitialLoad.value = true
    fetched.value = false
    simpleTimelineRef.value?.scrollToTop()
    if (locationId.value === null) return
    exploreStore.fetchLocationData(locationId.value).then(() => {
      fetched.value = true
      isInitialLoad.value = false
    })
  },
  { immediate: true },
)

useRefreshFunction(() => {
  if (locationId.value !== null) {
    exploreStore.fetchLocationData(locationId.value)
  }
})
</script>

<template>
  <div class="location-page">
    <simple-timeline
      ref="simpleTimeline"
      v-if="locationId !== null"
      :timeline-items="items"
      :view-link="`/explore/location/${locationId}/view/`"
    >
      <!-- Location Header Info -->
      <div class="location-header" v-if="details">
        <div class="location-header-right">
          <div class="location-title-row">
            <h1>{{ primaryName }}</h1>
          </div>
          <p class="location-subtitle-context" v-if="secondaryContext">
            <span>{{ secondaryContext }}</span>
          </p>
          <p class="location-meta">
            <span>{{ photoCountText(details.photoCount) }}</span>
          </p>
        </div>
      </div>

      <div class="loc-header" v-if="details">
        <thumbnail-img
          v-if="details.thumbnailId"
          :media-item-id="details.thumbnailId"
          :height="getThumbnailHeight(1080)"
          cover
          class="header-avatar-img"
        />
        <location-media-map :items="items" />
      </div>

      <!-- Loading State -->
      <div v-else class="loading-header">
        <v-lazy>
          <div class="center-loading">
            <v-progress-circular color="primary" indeterminate size="70" />
            <h2>Loading details...</h2>
          </div>
        </v-lazy>
      </div>

      <!-- Empty State -->
      <div class="empty-location" v-if="items.length === 0 && !isInitialLoad">
        <v-icon color="on-surface-variant" size="170" icon="mdi-map-marker-question-outline" />
        <h2>No media items found for this location</h2>
      </div>
    </simple-timeline>
  </div>
</template>

<style scoped>
.location-page {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
}

.location-header {
  display: flex;
  width: 100%;
  margin-bottom: 24px;
  align-items: center;
}

.loading-header {
  height: 196px;
  width: 100%;
  margin-bottom: 12px;
  display: flex;
  place-content: center;
  place-items: center;
}

.center-loading {
  text-align: center;
}

.center-loading h2 {
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 20px;
  margin-top: 10px;
}

.loc-header {
  display: flex;
  background-color: rgb(var(--v-theme-surface-container));
  margin-bottom: 25px;
  border-radius: 100px;
}

.header-avatar-img {
  width: 100%;
  height: 480px;
  object-fit: cover;
  border-radius: 50px;
  border: 15px solid rgb(var(--v-theme-surface-container));
  border-right: 0px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  margin-bottom: 0;
}

.location-header-right {
  min-width: 0;
  flex-grow: 1;
  padding: 20px;
}

.location-title-row {
  display: flex;
  align-items: flex-start;
}

.location-title-row h1 {
  min-width: 0;
  font-size: 44px;
  line-height: 1.2;
  font-weight: 700;
  margin: 0;
  overflow-wrap: anywhere;
  color: rgb(var(--v-theme-on-surface));
  margin-left: -2px;
}

.location-subtitle-context {
  margin: 4px 0 0;
  margin-top: 0;
  font-size: 1.15rem;
  font-weight: 500;
  color: rgb(var(--v-theme-primary));
}

.location-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 0;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 13px;
  font-weight: 500;
}

.empty-location {
  height: 560px;
  width: 100%;
  display: flex;
  place-items: center;
  place-content: center;
  flex-direction: column;
  color: rgb(var(--v-theme-on-surface-variant));
}

.empty-location h2 {
  font-weight: 500;
  margin: 20px 0 0;
}

@media (max-width: 720px) {
  .location-header {
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .location-header-right {
    padding: 10px;
    width: 100%;
  }

  .location-title-row {
    justify-content: center;
    align-items: center;
    flex-direction: column;
  }

  .location-title-row h1 {
    font-size: 34px;
  }

  .location-meta,
  .location-subtitle-context {
    justify-content: center;
  }
}
</style>
