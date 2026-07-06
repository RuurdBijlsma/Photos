<script setup lang="ts">
import { ref, computed, onUnmounted } from 'vue'
import { toHms } from '@/scripts/utils'

const props = withDefaults(
  defineProps<{
    modelValue: number
    max: number
    buffered?: Array<{ start: number; end: number }>
  }>(),
  {
    buffered: () => [],
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
}>()

const sliderRef = ref<HTMLElement | null>(null)
const showTooltip = ref(false)
const tooltipX = ref(0)
const hoverTime = ref(0)
const isDragging = ref(false)

// Convert buffered ranges to relative percentages against the total duration
const normalizedBuffered = computed(() => {
  if (!props.max || props.max <= 0) return []
  return props.buffered.map((range) => {
    const start = Math.max(0, Math.min(props.max, range.start))
    const end = Math.max(0, Math.min(props.max, range.end))
    const startPercent = (start / props.max) * 100
    const widthPercent = ((end - start) / props.max) * 100
    return { startPercent, widthPercent }
  })
})

const progressPercent = computed(() => {
  if (!props.max || props.max <= 0) return 0
  return Math.min(100, Math.max(0, (props.modelValue / props.max) * 100))
})

const formattedHoverTime = computed(() => {
  return toHms(hoverTime.value)
})

// Keep the tooltip pill centered on the pointer, while clamping its body within track boundaries
const tooltipStyle = computed(() => {
  if (!sliderRef.value) return { left: `${tooltipX.value}px` }
  const rect = sliderRef.value.getBoundingClientRect()
  const width = rect.width
  const halfTooltipWidth = 34 // approximate half width of "10:20" pill in pixels
  const clampedX = Math.max(halfTooltipWidth, Math.min(width - halfTooltipWidth, tooltipX.value))
  return {
    left: `${clampedX}px`,
  }
})

function getValFromEvent(e: PointerEvent): number {
  if (!sliderRef.value) return 0
  const rect = sliderRef.value.getBoundingClientRect()
  const offsetX = e.clientX - rect.left
  const percent = Math.min(1, Math.max(0, offsetX / rect.width))
  return percent * props.max
}

function onPointerEnter() {
  showTooltip.value = true
}

function onPointerMove(e: PointerEvent) {
  if (!sliderRef.value) return
  const rect = sliderRef.value.getBoundingClientRect()
  const offsetX = e.clientX - rect.left
  const clampedX = Math.min(rect.width, Math.max(0, offsetX))

  tooltipX.value = clampedX

  const percent = clampedX / rect.width
  hoverTime.value = percent * props.max

  if (isDragging.value) {
    emit('update:modelValue', percent * props.max)
  }
}

function onPointerLeave() {
  if (!isDragging.value) {
    showTooltip.value = false
  }
}

function onPointerDown(e: PointerEvent) {
  if (e.button !== 0 && e.pointerType === 'mouse') return // Left click only for mouse input
  isDragging.value = true
  showTooltip.value = true

  const val = getValFromEvent(e)
  emit('update:modelValue', val)

  window.addEventListener('pointermove', onGlobalPointerMove)
  window.addEventListener('pointerup', onGlobalPointerUp)
}

function onGlobalPointerMove(e: PointerEvent) {
  if (!isDragging.value || !sliderRef.value) return

  const rect = sliderRef.value.getBoundingClientRect()
  const offsetX = e.clientX - rect.left
  const clampedX = Math.min(rect.width, Math.max(0, offsetX))

  tooltipX.value = clampedX
  const percent = clampedX / rect.width
  hoverTime.value = percent * props.max

  emit('update:modelValue', percent * props.max)
}

function onGlobalPointerUp() {
  isDragging.value = false
  showTooltip.value = false
  window.removeEventListener('pointermove', onGlobalPointerMove)
  window.removeEventListener('pointerup', onGlobalPointerUp)
}

onUnmounted(() => {
  window.removeEventListener('pointermove', onGlobalPointerMove)
  window.removeEventListener('pointerup', onGlobalPointerUp)
})
</script>

<template>
  <div
    ref="sliderRef"
    class="custom-slider-container"
    @pointerenter="onPointerEnter"
    @pointermove="onPointerMove"
    @pointerleave="onPointerLeave"
    @pointerdown="onPointerDown"
  >
    <!-- Slate blue tooltip matching screenshot details -->
    <div v-if="showTooltip && max > 0" class="slider-tooltip" :style="tooltipStyle">
      {{ formattedHoverTime }}
    </div>

    <!-- Slider Track Wrap -->
    <div class="slider-track-wrap">
      <!-- Background Track -->
      <div class="slider-track-bg"></div>

      <!-- Buffered Segments -->
      <div
        v-for="(range, idx) in normalizedBuffered"
        :key="idx"
        class="slider-track-buffered"
        :style="{
          left: `${range.startPercent}%`,
          width: `${range.widthPercent}%`,
        }"
      ></div>

      <!-- Played/Progress Fill -->
      <div class="slider-track-fill" :style="{ width: `${progressPercent}%` }"></div>

      <!-- Red Drag Handle (Thumb) -->
      <div class="slider-thumb" :style="{ left: `${progressPercent}%` }"></div>
    </div>
  </div>
</template>

<style scoped>
.custom-slider-container {
  position: relative;
  width: 100%;
  height: 24px;
  display: flex;
  align-items: center;
  cursor: pointer;
  user-select: none;
  touch-action: none;
}

.slider-track-wrap {
  position: relative;
  width: 100%;
  height: 4px;
  background-color: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
  transition: height 0.1s ease;
}

/* Slick expansion animations similar to modern video players */
.custom-slider-container:hover .slider-track-wrap {
  height: 6px;
}

.slider-track-bg {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: transparent;
}

.slider-track-buffered {
  position: absolute;
  top: 0;
  height: 100%;
  background-color: rgba(255, 255, 255, 0.35);
  border-radius: 2px;
  pointer-events: none;
}

.slider-track-fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background-color: rgb(200, 0, 0);
  border-radius: 2px;
  pointer-events: none;
}

.slider-thumb {
  position: absolute;
  top: 50%;
  width: 12px;
  height: 12px;
  background-color: rgba(255, 0, 0, 0.9);
  border-radius: 50%;
  transform: translate(-50%, -50%) scale(0);
  transition: transform 0.1s ease;
  pointer-events: none;
  box-shadow: 0 0 4px rgba(0, 0, 0, 0.5);
}

.custom-slider-container:hover .slider-thumb,
.custom-slider-container:active .slider-thumb {
  transform: translate(-50%, -50%) scale(1);
}

.slider-tooltip {
  position: absolute;
  top: -32px;
  transform: translateX(-50%);
  background-color: rgba(0, 0, 0, 0.5);
  color: #ffffff;
  font-family: Jost, sans-serif;
  font-size: 13px;
  font-weight: 600;
  padding: 4px 14px;
  border-radius: 14px;
  white-space: nowrap;
  pointer-events: none;
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.25);
  z-index: 10;
}
</style>
