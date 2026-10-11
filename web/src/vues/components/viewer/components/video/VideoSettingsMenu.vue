<script setup lang="ts">
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiCogOutline from '~icons/mdi/cog-outline'
import { ref } from 'vue'

defineProps<{
  quality: number | 'source'
  sortedVideoSizes: number[]
  sourceHeight: number
  isSourceAvailable: boolean
  hasThumbnails: boolean
  currentPlaybackRate: number
  playbackRates: number[]
  attachFullscreen?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:quality', quality: number | 'source'): void
  (e: 'update:playbackRate', rate: number): void
}>()

const settingsMenuOpen = ref(false)

function onQualitySelect(size: number | 'source') {
  emit('update:quality', size)
  settingsMenuOpen.value = false
}

function onPlaybackRateSelect(rate: number) {
  emit('update:playbackRate', rate)
  settingsMenuOpen.value = false
}
</script>

<template>
  <v-menu
    v-if="hasThumbnails || isSourceAvailable"
    v-model="settingsMenuOpen"
    location="top center"
    :attach="attachFullscreen"
    :close-on-content-click="false"
  >
    <template #activator="{ props: menuProps }">
      <v-btn variant="plain" :icon="MdiCogOutline" rounded="xl" v-bind="menuProps" />
    </template>
    <v-list class="settings-menu-list">
      <!-- Submenu 1: Playback Speed -->
      <v-menu
        location="left top"
        open-on-hover
        :close-on-content-click="true"
        :attach="attachFullscreen"
      >
        <template #activator="{ props: speedMenuProps }">
          <v-list-item v-bind="speedMenuProps" class="menu-item-with-chevron">
            <v-list-item-title class="menu-text">Playback speed</v-list-item-title>
            <template #append>
              <span class="current-setting-label">{{ currentPlaybackRate }}x</span>
              <v-icon :icon="MdiChevronRight" size="small" class="ml-1" />
            </template>
          </v-list-item>
        </template>
        <v-list class="submenu-list">
          <v-list-item
            v-for="rate in playbackRates"
            :key="rate"
            :value="rate"
            @click="onPlaybackRateSelect(rate)"
            :active="currentPlaybackRate === rate"
          >
            <v-list-item-title class="menu-text">{{ rate }}x</v-list-item-title>
          </v-list-item>
        </v-list>
      </v-menu>

      <!-- Submenu 2: Quality -->
      <v-menu
        location="left top"
        open-on-hover
        :close-on-content-click="true"
        :attach="attachFullscreen"
      >
        <template #activator="{ props: qualityMenuProps }">
          <v-list-item v-bind="qualityMenuProps" class="menu-item-with-chevron">
            <v-list-item-title class="menu-text">Quality</v-list-item-title>
            <template #append>
              <span class="current-setting-label">
                {{ quality === 'source' ? `${sourceHeight}p` : `${quality}p` }}
              </span>
              <v-icon :icon="MdiChevronRight" size="small" class="ml-1" />
            </template>
          </v-list-item>
        </template>
        <v-list class="submenu-list">
          <v-list-item
            v-if="isSourceAvailable"
            value="source"
            @click="onQualitySelect('source')"
            :active="quality === 'source'"
          >
            <v-list-item-title class="menu-text">
              {{ sourceHeight }}p <span class="source-label">(source)</span>
            </v-list-item-title>
          </v-list-item>

          <template v-if="hasThumbnails">
            <v-list-item
              v-for="size in sortedVideoSizes"
              :key="size"
              :value="size"
              @click="onQualitySelect(size)"
              :active="quality === size"
            >
              <v-list-item-title class="menu-text">{{ size }}p</v-list-item-title>
            </v-list-item>
          </template>
        </v-list>
      </v-menu>
    </v-list>
  </v-menu>
  <v-btn v-else variant="plain" :icon="MdiCogOutline" rounded="xl" disabled />
</template>

<style scoped>
.settings-menu-list {
  min-width: 220px;
}

.submenu-list {
  min-width: 140px;
}

.menu-text {
  font-family: Jost, sans-serif;
  font-weight: 500;
}

.current-setting-label {
  font-family: Jost, sans-serif;
  font-size: 13px;
  opacity: 0.6;
}

.source-label {
  opacity: 0.6;
  font-size: 0.85em;
  font-weight: 400;
  margin-left: 4px;
}
</style>
