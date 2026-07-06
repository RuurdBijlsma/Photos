<script setup lang="ts">
import { useDailyCardStore } from '@/scripts/stores/timeline/dailyCardStore.ts'
import { computed, onMounted, watch, ref, nextTick } from 'vue'
import { useResizeObserver } from '@vueuse/core'
import DailyCard from '@/vues/components/timeline/daily-cards/DailyCard.vue'

defineProps<{
  width: number
}>()

const cardStore = useDailyCardStore()
const cards = computed(() => cardStore.todayCards)

const containerRef = ref<HTMLElement | null>(null)
const canScrollLeft = ref(false)
const canScrollRight = ref(false)

// Check scroll position to determine button visibility
function updateScrollButtons() {
  const el = containerRef.value
  if (!el) return
  canScrollLeft.value = el.scrollLeft > 1.5
  canScrollRight.value = el.scrollLeft + el.clientWidth < el.scrollWidth - 1.5
}

function scroll(direction: 'left' | 'right') {
  const el = containerRef.value
  if (!el) return
  const scrollAmount = el.clientWidth * 0.9
  const target = direction === 'left' ? el.scrollLeft - scrollAmount : el.scrollLeft + scrollAmount
  el.scrollTo({
    left: target,
    behavior: 'smooth',
  })
}

// Watch cards to update scroll buttons state after DOM rendering
watch(
  cards,
  () => {
    nextTick(() => {
      updateScrollButtons()
    })
  },
  { immediate: true },
)

// Automatically update state when container size changes
useResizeObserver(containerRef, () => {
  updateScrollButtons()
})

onMounted(() => cardStore.fetchDailyCards())
</script>

<template>
  <div class="daily-cards-wrapper" v-if="cards && cards.length > 0">
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

    <!-- Scrollable Cards Container -->
    <div class="daily-cards" ref="containerRef" @scroll="updateScrollButtons">
      <daily-card :card="card" :width="500" v-for="card in cards" :key="card.id" />
    </div>

    <!-- Scroll Right Button -->
    <transition name="fade">
      <v-btn
        v-if="canScrollRight"
        icon="mdi-chevron-right"
        class="scroll-btn scroll-btn-right"
        variant="flat"
        color="surface"
        rounded="xl"
        size="small"
        @click="scroll('right')"
      />
    </transition>
  </div>
</template>

<style scoped>
.daily-cards-wrapper {
  position: relative;
  width: calc(v-bind(width) * 1px);
  height: 300px;
}

.daily-cards {
  width: 100%;
  height: 100%;
  display: flex;
  gap: 20px;
  padding: 15px;
  padding-bottom: 0;
  overflow-x: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.daily-cards::-webkit-scrollbar {
  display: none;
}

/* Floating Navigation Buttons Styling */
.scroll-btn {
  position: absolute;
  top: calc(50% - 15px);
  z-index: 10;
}

.scroll-btn-left {
  left: 25px;
}

.scroll-btn-right {
  right: 25px;
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
