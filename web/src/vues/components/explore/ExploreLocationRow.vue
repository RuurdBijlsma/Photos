<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useResizeObserver } from '@vueuse/core'
import type { VisitedLocation } from '@/scripts/types/api/explore.ts'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'

const props = defineProps<{
  locations: VisitedLocation[]
}>()

const router = useRouter()
const containerRef = ref<HTMLElement | null>(null)
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

function updateScrollButtons() {
  const el = containerRef.value
  if (!el) return
  canScrollLeft.value = el.scrollLeft > 1.5
  canScrollRight.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1.5
}

function scroll(direction: 'left' | 'right') {
  const el = containerRef.value
  if (!el) return
  // Scroll about 75% of the visible container width for a smooth transition
  const scrollAmount = el.clientWidth * 0.75
  const target = direction === 'left' ? el.scrollLeft - scrollAmount : el.scrollLeft + scrollAmount
  el.scrollTo({
    left: target,
    behavior: 'smooth',
  })
}

// Smart context label generator
function getSecondaryLabel(loc: VisitedLocation): string {
  if (
    loc.admin1 &&
    loc.admin1 !== loc.name &&
    (loc.countryCode === 'US' || loc.countryCode === 'CA')
  ) {
    return loc.admin1
  }
  return loc.countryName || loc.admin1 || ''
}

useResizeObserver(containerRef, () => {
  updateScrollButtons()
})

watch(
  () => props.locations,
  () => {
    nextTick(() => {
      updateScrollButtons()
    })
  },
  { immediate: true },
)
</script>

<template>
  <div class="category-row-wrapper" v-if="locations && locations.length > 0">
    <div class="scroll-wrapper">
      <!-- Scroll Left Button -->
      <transition name="fade">
        <v-btn
          v-if="canScrollLeft"
          icon="mdi-chevron-left"
          class="scroll-btn scroll-btn-left"
          variant="elevated"
          color="surface"
          elevation="6"
          size="small"
          @click="scroll('left')"
        />
      </transition>

      <!-- Scrollable Container -->
      <div class="scroll-container" ref="containerRef" @scroll="updateScrollButtons">
        <router-link
          :to="`/explore/location/${loc.id}`"
          v-for="loc in locations"
          :key="loc.id"
          class="location-item"
        >
          <div class="avatar-wrapper">
            <thumbnail-img
              v-if="loc.thumbnailId"
              :media-item-id="loc.thumbnailId"
              :height="144"
              cover
              class="location-avatar"
            />
            <div v-else class="location-avatar-placeholder">
              <v-icon size="40" color="primary">mdi-map-marker-outline</v-icon>
            </div>
          </div>
          <span class="location-primary" :title="loc.name">{{ loc.name }}</span>
          <span class="location-secondary" :title="getSecondaryLabel(loc)">{{
            getSecondaryLabel(loc)
          }}</span>
        </router-link>
      </div>

      <!-- Scroll Right Button -->
      <transition name="fade">
        <v-btn
          v-if="canScrollRight"
          icon="mdi-chevron-right"
          class="scroll-btn scroll-btn-right"
          variant="elevated"
          color="surface"
          elevation="6"
          size="small"
          @click="scroll('right')"
        />
      </transition>
    </div>
  </div>
</template>

<style scoped>
.category-row-wrapper {
  margin-bottom: 0;
}

.scroll-wrapper {
  position: relative;
  width: 100%;
}

.scroll-container {
  display: flex;
  gap: 16px;
  overflow-x: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
  padding: 8px 4px;
}

.scroll-container::-webkit-scrollbar {
  display: none;
}

.location-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  cursor: pointer;
  width: 130px;
  flex-shrink: 0;
  border-radius: 16px;
  padding: 8px;
  transition:
    transform 0.2s cubic-bezier(0.16, 1, 0.3, 1),
    background-color 0.2s ease;
  user-select: none;
  text-decoration: none;
}

.location-item:active {
  background-color: rgba(var(--v-theme-on-surface), 0.1);
}

.location-item:hover {
  transform: translateY(-4px);
  background-color: rgba(var(--v-theme-on-surface), 0.04);
}

.avatar-wrapper {
  width: 110px;
  height: 110px;
  border-radius: 50%;
  overflow: hidden;
  border: 3px solid rgba(var(--v-theme-primary), 0.2);
  background-color: rgba(var(--v-theme-on-surface), 0.05);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 10px;
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.location-item:hover .avatar-wrapper {
  border-color: rgb(var(--v-theme-primary));
  box-shadow: 0 4px 10px rgba(var(--v-theme-primary), 0.2);
}

.location-avatar {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.location-avatar-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  width: 100%;
}

.location-primary {
  font-size: 0.85rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.location-secondary {
  font-size: 0.75rem;
  color: rgb(var(--v-theme-on-surface-variant));
  width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

/* Floating Scroll Buttons */
.scroll-btn {
  position: absolute;
  top: calc(50% - 30px);
  z-index: 5;
}

.scroll-btn-left {
  left: -12px;
}

.scroll-btn-right {
  right: -12px;
}

.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: scale(0.85);
}
</style>
