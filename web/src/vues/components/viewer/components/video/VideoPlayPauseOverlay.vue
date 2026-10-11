<script setup lang="ts">
import MdiPlay from '~icons/mdi/play'
import MdiPause from '~icons/mdi/pause'

defineProps<{
  action: 'play' | 'pause' | null
  trigger: number
}>()
</script>

<template>
  <div v-if="action" :key="trigger" class="play-pause-overlay">
    <div class="overlay-circle">
      <v-icon :icon="action === 'play' ? MdiPlay : MdiPause" size="40" />
    </div>
  </div>
</template>

<style scoped>
.play-pause-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 1505;
  pointer-events: none;
  animation: fade-out-overlay 750ms cubic-bezier(0.25, 1, 0.5, 1) forwards;
}

.overlay-circle {
  width: 80px;
  height: 80px;
  background-color: rgba(0, 0, 0, 0.3);
  color: rgba(255, 255, 255, 0.8);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
}

@keyframes fade-out-overlay {
  0% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(0.85);
  }
  15% {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1.1);
  }
  100% {
    opacity: 0;
    transform: translate(-50%, -50%) scale(1);
  }
}
</style>
