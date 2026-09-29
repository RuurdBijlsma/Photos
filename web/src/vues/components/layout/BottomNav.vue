<script setup lang="ts">
import { ref } from 'vue'
import MdiImage from '~icons/mdi/image'
import MdiImageAlbum from '~icons/mdi/image-album'
import MdiCompass from '~icons/mdi/compass'
import MdiMagnify from '~icons/mdi/magnify'

const currentTab = ref('photos')

const openSearch = () => {
  console.log('Search clicked')
}

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
</script>

<template>
  <div class="floating-nav-container">
    <v-sheet class="nav-sheet">
      <v-btn
        variant="text"
        v-for="tab in tabs"
        :key="tab.key"
        :prepend-icon="activeTab === tab.key ? tab.icon : undefined"
        :value="tab.key"
        rounded="pill"
        @click="activeTab = tab.key"
        >{{ tab.text }}</v-btn
      >
    </v-sheet>

    <v-btn :icon="MdiMagnify" @click="openSearch" />
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
  gap: 12px;
  max-width: calc(100vw - 32px);
}

.nav-sheet {
  width: 100%;
  display: flex;
  flex-direction: row;
  padding: 6px;
  border-radius:30px;
  font-size: 14px;
}
</style>
