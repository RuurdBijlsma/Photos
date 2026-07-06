<script setup lang="ts">
import { ref } from 'vue'
import type { StyleName } from '@/vues/components/map/BaseMap.vue'

export interface MapStyle {
  key: StyleName
  label: string
  thumb: string
}

defineProps<{
  currentStyle: StyleName
  mapMode: 'markers' | 'heatmap'
  nextStyle: MapStyle
  mapStyles: readonly MapStyle[]
}>()

const emit = defineEmits<{
  'update:currentStyle': [value: StyleName]
  'update:mapMode': [value: 'markers' | 'heatmap']
  'cycle-style': []
}>()

const isHovered = ref(false)

function handleStyleClick(key: StyleName) {
  emit('update:currentStyle', key)
}

function handleModeChange(newMode: 'markers' | 'heatmap') {
  emit('update:mapMode', newMode)
}

function handleCycleStyle() {
  emit('cycle-style')
}
</script>

<template>
  <div class="map-layer-selector" @mouseenter="isHovered = true" @mouseleave="isHovered = false">
    <v-fade-transition>
      <v-card v-show="isHovered" flat class="map-style-options-card" rounded="xl">
        <!-- Visualization View Selector -->
        <div class="map-mode-selector">
          <v-chip-group
            :model-value="mapMode"
            @update:model-value="handleModeChange"
            color="primary"
            class="ml-2"
            mandatory
            column
          >
            <v-chip
              prepend-icon="mdi-map-marker-multiple-outline"
              value="markers"
              key="markers"
              variant="flat"
            >
              Markers
            </v-chip>
            <v-chip prepend-icon="mdi-fire" value="heatmap" key="heatmap" variant="flat">
              Heatmap
            </v-chip>
          </v-chip-group>
        </div>

        <v-divider class="my-3 opacity-20" />

        <div class="map-style-options-list">
          <div
            v-for="style in mapStyles"
            :key="style.key"
            class="map-style-option"
            :class="{ active: currentStyle === style.key }"
            @click="handleStyleClick(style.key)"
          >
            <div class="map-style-option-thumb-wrapper">
              <v-img :src="style.thumb" cover class="map-style-option-thumb" :alt="style.label" />
              <v-icon
                v-if="currentStyle === style.key"
                color="primary"
                icon="mdi-check-circle"
                class="map-style-option-check"
                size="20"
              />
            </div>
            <span class="map-style-option-label">{{ style.label }}</span>
          </div>
        </div>
      </v-card>
    </v-fade-transition>

    <v-card class="map-layer-trigger-card" elevation="6" rounded="xl" @click="handleCycleStyle">
      <v-img :src="nextStyle.thumb" cover class="map-layer-trigger-thumb" :alt="nextStyle.label">
        <div class="map-layer-trigger-overlay">
          <span class="map-layer-trigger-label">Layers</span>
        </div>
      </v-img>
    </v-card>
  </div>
</template>

<style scoped>
.map-layer-selector {
  position: absolute;
  bottom: 12px;
  left: 12px;
  z-index: 2;
}

.map-layer-trigger-card {
  width: 96px;
  height: 96px;
  cursor: pointer;
  overflow: hidden;
  border: 2px solid white;
  transition:
    transform 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.map-layer-trigger-card:hover {
  transform: scale(1.05);
  border-color: rgba(var(--v-theme-primary), 0.8);
}

.map-layer-trigger-thumb {
  width: 100%;
  height: 100%;
}

.map-layer-trigger-overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: rgba(0, 0, 0, 0.45);
  color: white;
  text-align: center;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 0;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.6);
  pointer-events: none;
}

.map-style-options-card {
  position: absolute;
  bottom: 100px;
  left: 0;
  z-index: 3;
  padding: 16px 15px;
  background-color: rgba(var(--v-theme-background), 0.75);
  backdrop-filter: saturate(250%) blur(12px) !important;
  border: 1px solid rgba(var(--v-theme-primary), 0.2);
  width: max-content;
}

.map-style-options-card::after {
  content: '';
  position: absolute;
  bottom: -16px;
  left: 0;
  right: 0;
  height: 16px;
  background: transparent;
}

/* --- Visualization Toggle Styles --- */
.map-mode-selector {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.map-style-options-list {
  display: flex;
  gap: 12px;
}

.map-style-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  width: 68px;
  text-align: center;
}

.map-style-option-thumb-wrapper {
  position: relative;
  width: 52px;
  height: 52px;
  border-radius: 15px;
  overflow: hidden;
  border: 2px solid transparent;
  transition:
    transform 0.2s ease,
    border-color 0.2s ease;
}

.map-style-option-thumb {
  width: 100%;
  height: 100%;
}

.map-style-option:hover .map-style-option-thumb-wrapper {
  transform: translateY(-2px);
  border-color: rgba(var(--v-theme-primary), 0.5);
}

.map-style-option.active .map-style-option-thumb-wrapper {
  border-color: rgb(var(--v-theme-primary));
  box-shadow: 0 0 0 2px rgba(var(--v-theme-primary), 0.2);
}

.map-style-option-check {
  position: absolute;
  top: 2px;
  right: 2px;
  background: rgba(var(--v-theme-surface-variant), 1);
  border-radius: 50%;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
}

.map-style-option-label {
  font-size: 11px;
  font-weight: 500;
  color: #374151;
  white-space: nowrap;
  transition: color 0.2s ease;
}

.map-style-option:hover .map-style-option-label {
  color: rgb(var(--v-theme-primary));
}

.map-style-option.active .map-style-option-label {
  font-weight: 700;
  color: rgb(var(--v-theme-primary));
}
</style>
