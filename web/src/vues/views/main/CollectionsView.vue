<script setup lang="ts">
import MdiCamera from '~icons/mdi/camera'
import MdiImageAlbum from '~icons/mdi/image-album'
import MdiMapOutline from '~icons/mdi/map-outline'
import MdiTrashCanOutline from '~icons/mdi/trash-can-outline'
import MdiFaceManShimmerOutline from '~icons/mdi/face-man-shimmer-outline'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'

const systemStore = useSystemStore()

const collections = [
  { icon: MdiMapOutline, title: 'Map', to: '/map' },
  { icon: MdiTrashCanOutline, title: 'Bin', to: '/bin' },
  { icon: MdiCamera, title: 'Cameras', to: '/cameras' },
  { icon: MdiImageAlbum, title: 'Albums', to: '/albums' },
]
</script>

<template>
  <main-layout-container>
    <div class="collections-container">
      <header class="collections-header">
        <h1>Collections</h1>
      </header>

      <div class="collections-grid">
        <router-link
          v-for="item in collections"
          :key="item.to"
          :to="item.to"
          class="collection-card"
        >
          <v-icon :icon="item.icon" size="28" class="collection-icon" />
          <span class="collection-title">{{ item.title }}</span>
          <v-icon icon="mdi-chevron-right" size="20" class="collection-chevron" />
        </router-link>

        <router-link
          v-if="systemStore.stats.hasClusteredPeople"
          to="/people"
          class="collection-card"
        >
          <v-icon :icon="MdiFaceManShimmerOutline" size="28" class="collection-icon" />
          <span class="collection-title">People</span>
          <v-icon icon="mdi-chevron-right" size="20" class="collection-chevron" />
        </router-link>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.collections-container {
  padding: 20px 16px;
}

.collections-header h1 {
  font-size: 2rem;
  font-weight: 600;
  margin: 0 0 24px;
}

.collections-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.collection-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 18px;
  border-radius: 20px;
  color: inherit;
  text-decoration: none;
  background: rgb(var(--v-theme-surface-container-low));
  transition:
    background-color 0.18s ease,
    transform 0.18s ease;
}

.collection-card:active {
  transform: scale(0.98);
}

.collection-icon {
  color: rgb(var(--v-theme-primary));
}

.collection-title {
  flex: 1;
  font-size: 1rem;
  font-weight: 500;
}

.collection-chevron {
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.6;
}
</style>
