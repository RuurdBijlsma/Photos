<script setup lang="ts">
import { onMounted } from 'vue'
import { useExploreStore } from '@/scripts/stores/exploreStore.ts'
import ExploreLocationRow from '@/vues/components/explore/ExploreLocationRow.vue'

const exploreStore = useExploreStore()

onMounted(async () => {
  if (!exploreStore.visitedPlaces) {
    await exploreStore.fetchVisitedPlaces()
  }
})
</script>

<template>
  <v-card class="explore-locations-card" flat :loading="exploreStore.isVisitedPlacesLoading">
    <div class="card-header">
      <div class="header-texts">
        <h3 class="card-title">Frequently visited places</h3>
        <p class="card-subtitle">
          Explore photos and videos from some of these highlighted locations.
        </p>
      </div>
    </div>

    <!-- Places Row -->
    <div
      v-if="exploreStore.visitedPlaces && exploreStore.visitedPlaces.length > 0"
      class="locations-list"
    >
      <explore-location-row :locations="exploreStore.visitedPlaces" />
    </div>
  </v-card>
</template>

<style scoped>
.explore-locations-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 28px !important;
  padding: 24px;
  border: none !important;
  overflow: hidden;
  min-height:288px;
}

.card-header {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 24px;
}

.header-texts {
  display: flex;
  flex-direction: column;
}

.card-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.card-subtitle {
  margin: 4px 0 0;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 35px;
}

.loading-text {
  margin-top: 16px;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.95rem;
}

.locations-list {
  display: flex;
  flex-direction: column;
}
</style>
