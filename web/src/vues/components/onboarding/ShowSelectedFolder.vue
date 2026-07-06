<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  folder: string[]
  pill: boolean
  bgColor?: string
  textColor?: string
  iconColor?: string
  includeSelectedText?: boolean
  excludeCheckIcon?: boolean
}>()

const backgroundColor = computed(() => {
  if (!props.pill) return undefined
  if (props.bgColor) return `rgb(var(--v-theme-${props.bgColor}))`
  return `rgb(var(--v-theme-secondary-container))`
})
const color = computed(() => {
  if (!props.pill) return undefined
  if (props.textColor) return `rgb(var(--v-theme-${props.textColor}))`
  return `rgb(var(--v-theme-on-secondary-container))`
})
</script>

<template>
  <div class="folder-selection text-lg-caption">
    <v-icon
      v-if="!excludeCheckIcon"
      :color="iconColor ?? 'primary'"
      icon="mdi-check-circle-outline"
    />
    <span class="primary-color" v-if="includeSelectedText">Selected folder:</span>
    <div
      :style="{
        backgroundColor,
        color,
        paddingLeft: pill ? '20px' : '0',
      }"
      class="viewed-folder"
    >
      <span class="opa">Root</span>
      <template v-for="(component, index) in folder" :key="index">
        <v-icon color="on-secondary-container" icon="mdi-chevron-right" />
        <span>{{ component }}</span>
      </template>
    </div>
  </div>
</template>

<style scoped>
.folder-selection {
  display: flex;
  justify-content: center;
  gap: 10px;
  align-items: center;
  border-radius: 20px;
  padding-left: 10px;
}

.primary-color {
  color: rgb(var(--v-theme-primary));
}

.viewed-folder {
  font-weight: 500;
  display: flex;
  align-items: center;
  padding: 5px 20px;
  border-radius: 20px;
}

.opa {
  opacity: 0.7;
  font-weight: bold;
}
</style>
