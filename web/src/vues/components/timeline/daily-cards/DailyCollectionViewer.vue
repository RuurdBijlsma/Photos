<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useWindowSize } from '@vueuse/core'
import type { DailyCardResponse } from '@/scripts/types/api/dailyCards.ts'
import { useDailyCardStore } from '@/scripts/stores/timeline/dailyCardStore.ts'
import { getThumbnailHeight, getVideoHeight, makeDateTimeString } from '@/scripts/utils.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'

const props = defineProps<{
  card: DailyCardResponse
}>()

const cardStore = useDailyCardStore()
const router = useRouter()
const windowSize = useWindowSize()

const mediaItems = computed(() => cardStore.getPayloadItems(props.card) || [])

// Slideshow state
const currentIndex = ref(0)
const progress = ref(0)
const isPaused = ref(false)
const pausedByScroll = ref(false)
let timerId: ReturnType<typeof setInterval> | null = null

// Precise timer tracking for pausing/resuming
const elapsedTimeMs = ref(0)
let currentIntervalStartTime = 0

const videoRef = ref<HTMLVideoElement | null>(null)

const currentItem = computed(() => {
  if (mediaItems.value.length === 0) return null
  return mediaItems.value[currentIndex.value]
})

const nextItem = computed(() => {
  if (mediaItems.value.length <= 1) return null
  const nextIdx = (currentIndex.value + 1) % mediaItems.value.length
  return mediaItems.value[nextIdx]
})

// Delay configurations based on media type
const currentDuration = computed(() => {
  const item = currentItem.value
  if (item && item.isVideo) {
    return item.durationMs ?? 10000
  }
  return 5000 // 5 seconds for photos
})

const currentImageUrl = computed(() => {
  const item = currentItem.value
  if (!item) return ''
  return mediaItemService.getPhotoThumbnail(
    item.id,
    getThumbnailHeight(windowSize.height.value),
    !item.hasThumbnails,
  )
})

const itemDateString = computed(() => {
  const item = currentItem.value
  if (!item || !item.takenAtLocal) return null
  return makeDateTimeString(new Date(item.takenAtLocal))
})

const nextImageUrl = computed(() => {
  const item = nextItem.value
  if (!item || item.isVideo) return ''
  return mediaItemService.getPhotoThumbnail(
    item.id,
    getThumbnailHeight(windowSize.height.value),
    !item.hasThumbnails,
  )
})

const currentVideoUrl = computed(() => {
  const item = currentItem.value
  if (!item) return ''
  return mediaItemService.getVideo(
    item.id,
    getVideoHeight(windowSize.height.value),
    !item.hasThumbnails,
  )
})

// Slideshow Control
function startTimer(isResume = false) {
  stopTimer()
  if (isPaused.value || mediaItems.value.length === 0) return

  if (!isResume) {
    elapsedTimeMs.value = 0
    progress.value = 0
  }

  const duration = currentDuration.value
  currentIntervalStartTime = Date.now()

  timerId = setInterval(() => {
    const sessionElapsed = Date.now() - currentIntervalStartTime
    const totalElapsed = elapsedTimeMs.value + sessionElapsed
    progress.value = Math.min((totalElapsed / duration) * 100, 100)

    if (totalElapsed >= duration) {
      elapsedTimeMs.value = 0
      nextSlide()
    }
  }, 30)
}

function stopTimer() {
  if (timerId) {
    clearInterval(timerId)
    timerId = null
  }
  if (currentIntervalStartTime > 0) {
    elapsedTimeMs.value += Date.now() - currentIntervalStartTime
    currentIntervalStartTime = 0
  }
}

function togglePause() {
  isPaused.value = !isPaused.value
  if (isPaused.value) {
    stopTimer()
    videoRef.value?.pause()
  } else {
    startTimer(true)
    videoRef.value?.play().catch(() => {})
  }
}

function prevSlide() {
  if (mediaItems.value.length === 0) return
  if (currentIndex.value > 0) {
    currentIndex.value--
  } else {
    currentIndex.value = mediaItems.value.length - 1
  }
}

function nextSlide() {
  if (mediaItems.value.length === 0) return
  if (currentIndex.value < mediaItems.value.length - 1) {
    currentIndex.value++
  } else {
    currentIndex.value = 0
  }
}

function goBack() {
  router.push('/')
}

function scrollToGrid() {
  const el = document.getElementById('collection-grid')
  if (el) {
    el.scrollIntoView({ behavior: 'smooth' })
  }
}

// Pause slideshow when scrolling down
function onScroll(event: Event) {
  const target = event.target as HTMLElement
  if (target.scrollTop > 100) {
    if (!isPaused.value) {
      isPaused.value = true
      pausedByScroll.value = true
      stopTimer()
      videoRef.value?.pause()
    }
  } else {
    if (pausedByScroll.value) {
      isPaused.value = false
      pausedByScroll.value = false
      startTimer(true)
      videoRef.value?.play().catch(() => {})
    }
  }
}

// State Watchers
watch(currentIndex, () => {
  startTimer()
  nextTick(() => {
    if (videoRef.value) {
      if (isPaused.value) {
        videoRef.value.pause()
      } else {
        videoRef.value.play().catch(() => {})
      }
    }
  })
})

watch(
  mediaItems,
  (newItems) => {
    if (newItems && newItems.length > 0) {
      currentIndex.value = 0
      startTimer()
    }
  },
  { immediate: true },
)

// Keyboard Controls
function handleKeyDown(event: KeyboardEvent) {
  if (event.key === 'ArrowLeft') {
    prevSlide()
  } else if (event.key === 'ArrowRight') {
    nextSlide()
  } else if (event.key === ' ') {
    event.preventDefault()
    togglePause()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
  startTimer()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
  stopTimer()
})
</script>

<template>
  <div class="collection-container" @scroll="onScroll">
    <div class="empty-collection" v-if="mediaItems.length === 0">
      <v-icon size="100" icon="mdi-image-multiple-outline" color="grey" />
      <p>This collection has no media items.</p>
      <v-btn @click="goBack" color="primary">Go Back</v-btn>
    </div>

    <template v-else>
      <!-- Story Slideshow Section -->
      <div class="story-section">
        <!-- Controls & Indicators Up Top -->
        <div class="story-header">
          <div class="story-top-controls">
            <!-- Left Column: Close Button on top left -->
            <div class="control-left">
              <v-btn
                icon="mdi-close"
                variant="text"
                color="white"
                size="small"
                class="close-btn"
                @click="goBack"
              />
              <div class="story-title-bar">
                <div class="story-card-info">
                  <h3>{{ card.title }}</h3>
                  <p v-if="card.subtitle" class="subtitle">{{ card.subtitle }}</p>
                  <!--                  <p class="subtitle">What's up gang, how you doing?</p>-->
                </div>
              </div>
            </div>

            <!-- Center Column: Centered Play/Pause Button and compact progress bars -->
            <div class="control-center">
              <div class="story-progress-wrapper">
                <v-btn
                  :icon="isPaused ? 'mdi-play' : 'mdi-pause'"
                  variant="text"
                  color="white"
                  size="small"
                  class="pause-btn"
                  @click="togglePause"
                />
                <div class="story-progress-container">
                  <div
                    v-for="(item, index) in mediaItems"
                    :key="item.id"
                    class="story-progress-bar-bg"
                  >
                    <div
                      class="story-progress-bar-fill"
                      :style="{
                        width:
                          index < currentIndex
                            ? '100%'
                            : index === currentIndex
                              ? `${progress}%`
                              : '0%',
                      }"
                    ></div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Right Column: Empty Spacer to maintain center alignment -->
            <div class="control-right"></div>
          </div>
          <div class="story-top-subline" v-if="itemDateString">
            <span>{{ itemDateString }}</span>
          </div>
        </div>

        <!-- Slideshow Player -->
        <div class="story-media-container" v-if="currentItem">
          <!-- Background Glow Effect -->
          <div
            class="blurry-bg"
            :style="{
              backgroundImage: `url(${currentImageUrl})`,
            }"
            v-if="!currentItem.isVideo"
          ></div>

          <!-- Touch/Click Zones -->
          <div class="click-overlay-left" @click="prevSlide"></div>
          <div class="click-overlay-right" @click="nextSlide"></div>

          <div class="media-wrapper">
            <video
              ref="videoRef"
              @click="togglePause"
              v-if="currentItem.isVideo"
              :src="currentVideoUrl"
              autoplay
              muted
              class="story-media"
              @ended="nextSlide"
            />
            <img v-else :src="currentImageUrl" class="story-media" alt="Story Slide" />
          </div>

          <!-- Preload buffer image -->
          <img
            v-if="nextImageUrl"
            :src="nextImageUrl"
            style="display: none"
            alt="Preload next slide"
          />
        </div>

        <!-- Scroll down indicator -->
        <div class="scroll-down-hint" @click="scrollToGrid">
          <v-icon icon="mdi-chevron-down" size="28" />
          <span>Scroll down for all photos</span>
        </div>
      </div>

      <!-- Scrollable Grid Section -->
      <div class="grid-section" id="collection-grid">
        <div class="grid-header">
          <h2>{{ card.title }}</h2>
          <p v-if="card.subtitle" class="grid-subtitle">{{ card.subtitle }}</p>
          <span class="grid-count"
            >{{ mediaItems.length }} item{{ mediaItems.length === 1 ? '' : 's' }}</span
          >
        </div>

        <simple-timeline
          class="daily-timeline"
          hide-scroll-bar
          :timeline-items="mediaItems"
          :view-link="`/daily/${card.id}/view/`"
          :context="{}"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.collection-container {
  overflow-y: auto;
  height: 100%;
  width: 100%;
  scroll-behavior: smooth;
  background-color: #0c0c0e;
  color: #ffffff;
}

.collection-container::-webkit-scrollbar {
  width: 8px;
}
.collection-container::-webkit-scrollbar-track {
  background: #111;
}
.collection-container::-webkit-scrollbar-thumb {
  background: #333;
  border-radius: 4px;
}

.empty-collection {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 16px;
  color: #888;
}

.story-section {
  position: relative;
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  background-color: #000;
  overflow: hidden;
  border-bottom: 1px solid #1c1c1e;
}

.story-header {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  z-index: 20;
  background: linear-gradient(
    180deg,
    rgba(0, 0, 0, 0.95) 0%,
    rgba(0, 0, 0, 0.5) 70%,
    rgba(0, 0, 0, 0) 100%
  );
  padding-top: 15px;
  padding-bottom: 25px;
}

.story-top-controls {
  display: grid;
  grid-template-columns: 80px 1fr 80px;
  align-items: center;
  padding: 0 16px;
  width: 100%;
}

.story-top-subline {
  width: 100%;
  display: flex;
  justify-content: center;
}

.control-left {
  display: flex;
  justify-content: flex-start;
  width: 300px;
}

.control-center {
  display: flex;
  justify-content: center;
}

.control-right {
  display: flex;
  justify-content: flex-end;
  width: 80px;
}

.story-progress-wrapper {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  max-width: 720px;
}

.story-progress-container {
  display: flex;
  gap: 5px;
  flex-grow: 1;
}

.story-progress-bar-bg {
  flex: 1;
  height: 3px;
  background-color: rgba(255, 255, 255, 0.25);
  border-radius: 1.5px;
  overflow: hidden;
}

.story-progress-bar-fill {
  height: 100%;
  background-color: #ffffff;
  width: 0%;
  transition: width 0.03s linear;
}

.pause-btn {
  background-color: rgba(0, 0, 0, 0.4);
  min-width: unset;
  padding: 0;
  width: 32px;
  height: 32px;
}

.story-title-bar {
  display: flex;
  justify-content: flex-start;
  align-items: center;
  margin-left: 15px;
}

.story-card-info h3 {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 600;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8);
}

.story-card-info .subtitle {
  margin: 2px 0 0 0;
  font-size: 0.85rem;
  opacity: 0.8;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
}

.story-media-container {
  flex-grow: 1;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.blurry-bg {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-size: cover;
  background-position: center;
  filter: blur(40px) brightness(40%);
  opacity: 0.75;
  z-index: 1;
}

.click-overlay-left {
  position: absolute;
  top: 0;
  left: 0;
  width: 40%;
  height: 100%;
  z-index: 15;
  cursor: pointer;
}

.click-overlay-right {
  position: absolute;
  top: 0;
  right: 0;
  width: 60%;
  height: 100%;
  z-index: 15;
  cursor: pointer;
}

.media-wrapper {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.story-media {
  width: 100%;
  height: 100%;
  object-fit: contain;
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.scroll-down-hint {
  position: absolute;
  bottom: 12px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  z-index: 18;
  cursor: pointer;
  opacity: 0.75;
  transition:
    opacity 0.2s,
    transform 0.2s;
  font-size: 0.8rem;
  font-weight: 500;
  letter-spacing: 1px;
}

.scroll-down-hint:hover {
  opacity: 1;
  transform: translateX(-50%) translateY(2px);
}

.scroll-down-hint span {
  text-shadow: 0 1px 4px rgba(0, 0, 0, 1);
}

.grid-section {
  padding: 30px 20px 0;
  background-color: rgb(var(--v-theme-background));
}

.grid-header {
  margin-bottom: 24px;
}

.grid-header h2 {
  font-size: 1.85rem;
  font-weight: 600;
  margin: 0;
  color: rgb(var(--v-theme-on-background));
}

.grid-subtitle {
  margin: 6px 0;
  font-size: 1.1rem;
  opacity: 0.7;
  color: rgb(var(--v-theme-on-background));
}

.grid-count {
  font-size: 0.9rem;
  opacity: 0.5;
  color: rgb(var(--v-theme-on-background));
}

.daily-timeline {
  height: 75vh;
}
</style>
