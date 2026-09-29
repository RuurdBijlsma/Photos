<script setup lang="ts">
import { ref } from 'vue'
import MdiImage from '~icons/mdi/image'
import MdiImageAlbum from '~icons/mdi/image-album'
import MdiCompass from '~icons/mdi/compass'
import MdiMagnify from '~icons/mdi/magnify'

const activeTab = ref('photos')

const tabs = [
  {
    icon: MdiImage,
    text: 'Photos',
    key: 'photos',
  },
  {
    icon: MdiImageAlbum,
    text: 'Collections',
    key: 'collections',
  },
  {
    icon: MdiCompass,
    text: 'Explore',
    key: 'explore',
  },
]

const openSearch = () => {
  console.log('Search clicked')
}
</script>

<template>
  <div class="floating-nav-container">
    <v-sheet class="nav-sheet" elevation="3">
      <v-btn
        v-for="tab in tabs"
        :key="tab.key"
        :variant="activeTab === tab.key ? 'flat' : 'text'"
        :color="activeTab === tab.key ? 'surface-variant' : undefined"
        :prepend-icon="activeTab === tab.key ? tab.icon : undefined"
        :class="[
          'tab-btn text-none',
          activeTab === tab.key ? 'tab-btn--active' : 'tab-btn--inactive',
        ]"
        rounded="pill"
        @click="activeTab = tab.key"
      >
        {{ tab.text }}
      </v-btn>
    </v-sheet>

    <v-btn :icon="MdiMagnify" class="search-btn" @click="openSearch" elevation="3" />
  </div>
</template>

<style scoped>
.floating-nav-container {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  display: flex;
  align-items: center;
  gap: 10px;
  width: max-content;
  max-width: calc(100vw - 32px);
}

.nav-sheet {
  align-items: center;
  padding: 6px;
  border-radius: 50px;
}

.tab-btn {
  font-size: 13px;
  font-weight: 500;
  letter-spacing: 0.1px;
  transition: all 0.2s ease;
}

.tab-btn--active {
  padding: 0 16px !important;
}

.tab-btn--inactive {
  padding: 0 12px !important;
  opacity: 0.8;
}

.tab-btn:hover {
  opacity: 1;
}
</style>
