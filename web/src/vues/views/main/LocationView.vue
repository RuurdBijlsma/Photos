<script setup lang="ts">
import MdiImageOutline from '~icons/mdi/image-outline'
import MdiMapMarkerQuestionOutline from '~icons/mdi/map-marker-question-outline'
import { computed, ref, useTemplateRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import LocationMediaMap from '@/vues/components/map/LocationMediaMap.vue'
import { useRefreshFunction } from '@/scripts/composables/useRefreshFunction.ts'
import { getThumbnailHeight } from '@/scripts/utils.ts'
import { usePageTitle } from '@/scripts/composables/usePageTitle.ts'

const route = useRoute()
const router = useRouter()
const exploreStore = useExploreStore()
const simpleTimelineRef = useTemplateRef('simpleTimeline')

const isInitialLoad = ref(true)
const fetched = ref(false)

const locationId = computed(() => {
  const rawId = route.params.locationId
  const idStr = Array.isArray(rawId) ? rawId[0] : rawId
  return idStr || null
})

const isPlace = computed(() => !locationId.value?.includes(':'))

watch(
  isPlace,
  () => {
    console.log('isPlace', isPlace.value)
  },
  { immediate: true },
)

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

const secondaryContextLinks = computed(() => {
  if (!details.value) return []
  const links = []
  const countryCode = (details.value.countryCode || '').toUpperCase()

  // 1. Generate Link for Admin1
  if (details.value.admin1 && details.value.admin1 !== details.value.name) {
    const id = `admin1:${countryCode}:${details.value.admin1}`
    links.push({
      text: details.value.admin1,
      to: `/explore/location/${id}`,
      id,
    })
  }

  // 2. Generate Link for Country
  if (details.value.countryName && details.value.countryName !== details.value.name) {
    const id = `country:${countryCode}`
    links.push({
      text: details.value.countryName,
      to: `/explore/location/${id}`,
      id,
    })
  }

  return links
})

function prefetchLocation(id: string) {
  if (id && !exploreStore.locations.has(id)) {
    exploreStore.fetchLocationData(id)
  }
}

function handleSelectLocation(targetLocationId: string) {
  if (targetLocationId && targetLocationId !== locationId.value) {
    router.push(`/explore/location/${targetLocationId}`)
  }
}

function handleHoverLocation(targetLocationId: string) {
  prefetchLocation(targetLocationId)
}

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
usePageTitle(primaryName, { fallback: 'Place' })
</script>

<template>
  <div class="location-page">
    <simple-timeline
      ref="simpleTimeline"
      v-if="locationId !== null"
      :timeline-items="items"
      :view-link="`/explore/location/${locationId}/view/`"
    >
      <div class="loc-header" v-if="details">
        <!-- Overlay Container for Thumbnail and Metadata -->
        <div class="header-avatar-container">
          <thumbnail-img
            v-if="details.thumbnailId"
            :media-item-id="details.thumbnailId"
            :height="getThumbnailHeight(1080)"
            cover
            class="header-avatar-img"
          />
          <div v-else class="header-avatar-placeholder">
            <v-icon size="64" color="on-surface-variant" :icon="MdiImageOutline" />
          </div>

          <v-theme-provider theme="dark" with-background>
            <div class="header-overlay">
              <div class="overlay-content">
                <h1 class="location-title">{{ primaryName }}</h1>
                <p class="location-subtitle" v-if="secondaryContextLinks.length > 0">
                  <template v-for="(link, index) in secondaryContextLinks" :key="link.to">
                    <router-link
                      :to="link.to"
                      class="context-link"
                      @mouseenter="prefetchLocation(link.id)"
                    >
                      {{ link.text }}
                    </router-link>
                    <span v-if="index < secondaryContextLinks.length - 1">, </span>
                  </template>
                </p>
                <p class="location-meta">
                  {{ photoCountText(details.photoCount) }}
                </p>
              </div>
            </div>
          </v-theme-provider>
        </div>

        <location-media-map
          :is-place="isPlace"
          :items="items"
          @select-location="handleSelectLocation"
          @hover-location="handleHoverLocation"
        />
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
        <v-icon color="on-surface-variant" size="170" :icon="MdiMapMarkerQuestionOutline" />
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

/* Container hosting both the image and the text overlay */
.header-avatar-container {
  position: relative;
  width: 100%;
  height: 480px;
  overflow: hidden;
  border-radius: 50px;
  border: 15px solid rgb(var(--v-theme-surface-container));
  border-right: 0;
  background-color: rgb(var(--v-theme-surface-bright));
}

.header-avatar-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.header-avatar-placeholder {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
}

/* Subtle gradient to support white-text readability on arbitrary image content */
.header-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    to bottom,
    rgba(var(--v-theme-background), 0) 35%,
    rgba(var(--v-theme-background), 0.5) 60%,
    rgba(var(--v-theme-background), 0.8) 100%
  );
  display: flex;
  align-items: flex-end;
  padding: 25px;
  pointer-events: none;
}

.overlay-content {
  color: #ffffff;
  width: 100%;
}

.location-title {
  font-size: 38px;
  font-weight: 600;
  line-height: 1.2;
  margin: 0;
  overflow-wrap: anywhere;
  color: rgb(var(--v-theme-on-background));
}

.location-subtitle {
  margin: 4px 0 0;
  font-size: 1.15rem;
  font-weight: 500;
  color: rgb(var(--v-theme-primary));
}

.context-link {
  color: rgb(var(--v-theme-primary));
  text-decoration: none;
  pointer-events: auto; /* Enable clicks since .header-overlay has pointer-events: none */
}

.context-link:hover {
  text-decoration: underline;
}

.location-meta {
  margin: 8px 0 0;
  font-size: 13px;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface-variant));
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
  .loc-header {
    flex-direction: column;
    border-radius: 40px;
  }

  .header-avatar-container {
    border-right: 15px solid rgb(var(--v-theme-surface-container));
    border-bottom: 0;
    border-radius: 40px 40px 0 0;
    height: 360px;
  }

  .header-overlay {
    padding: 20px;
  }

  .location-title {
    font-size: 28px;
  }

  .location-subtitle {
    font-size: 1rem;
  }
}
</style>
