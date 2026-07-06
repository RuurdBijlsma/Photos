<script setup lang="ts">
withDefaults(
  defineProps<{
    src: string
    height?: number
    width?: number
    maxWidth?: number
    borderRadius?: string
    strength?: number
  }>(),
  {
    borderRadius: '0',
    strength: 1,
  },
)
const emit = defineEmits(['error'])
</script>

<template>
  <div class="glow-image">
    <img
      :height="height"
      :width="width"
      :style="{
        maxWidth: maxWidth === undefined ? undefined : maxWidth + 'px',
      }"
      @error="(e) => emit('error', e)"
      aria-hidden="true"
      alt="Album thumbnail"
      class="image-bg"
      :src="src"
    />
    <img
      :height="height"
      :width="width"
      :style="{
        maxWidth: maxWidth === undefined ? undefined : maxWidth + 'px',
      }"
      alt="Album thumbnail"
      class="image-first"
      :src="src"
    />
  </div>
</template>

<style scoped>
.glow-image {
  position: relative;
}

.image-first {
  position: relative;
  border-radius: v-bind(borderRadius);
  box-shadow: 0 20px 30px 0 rgba(0, 0, 0, 0.2);
  z-index: 5;
  object-fit: cover;
}

.image-bg {
  z-index: 2;
  position: absolute;
  filter: blur(100px) brightness(80%);
  pointer-events: none;
  object-fit: cover;
  opacity: v-bind(strength);
}
</style>
