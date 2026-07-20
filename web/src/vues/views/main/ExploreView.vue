<script setup lang="ts">
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import ExploreTable from '@/vues/components/explore/ExploreTable.vue'
import ExploreHistograms from '@/vues/components/explore/ExploreHistograms.vue'
import ExploreLocations from '@/vues/components/explore/ExploreLocations.vue'
</script>

<template>
  <main-layout-container class="explore-scroll-view">
    <teleport to="body">
      <router-view />
    </teleport>

    <div class="explore-content">
      <header class="explore-header mb-6">
        <h1 class="explore-title">Explore Insights</h1>
        <p class="explore-subtitle">
          Explore stats from your library, from seasonal weather conditions and geographic metadata
          to activity patterns over time.
        </p>
      </header>

      <explore-locations class="mb-7" />

      <explore-histograms />

      <explore-table />
    </div>
  </main-layout-container>
</template>

<style scoped>
.explore-scroll-view {
  overflow-y: auto;
}

.explore-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 32px 24px;
}

.explore-title {
  font-size: 2.125rem;
  font-weight: 700;
  margin-bottom: 6px;
  color: rgb(var(--v-theme-on-surface));
}

.explore-subtitle {
  font-size: 0.95rem;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 0;
}
</style>
