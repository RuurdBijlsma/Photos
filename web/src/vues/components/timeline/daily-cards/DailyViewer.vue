<script setup lang="ts">
import { useDailyCardStore } from '@/scripts/stores/timeline/dailyCardStore.ts'
import { computed, defineAsyncComponent, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import DailyCollectionViewer from '@/vues/components/timeline/daily-cards/DailyCollectionViewer.vue'

const LocationEstimatr = defineAsyncComponent(
  () => import('@/vues/components/timeline/daily-cards/LocationEstimatr.vue'),
)

const route = useRoute()
const router = useRouter()
const snackbarStore = useSnackbarsStore()
const cardStore = useDailyCardStore()

const cardId = computed(() => {
  const rawId = route.params.cardId
  if (rawId && !Array.isArray(rawId)) {
    const idNum = +rawId
    if (isNaN(idNum)) return null
    return idNum
  }
  return null
})

const card = computed(() => {
  if (cardId.value === null) return null
  return cardStore.cardsById.get(cardId.value) ?? null
})

watch(
  card,
  () => {
    if (card.value === null) {
      // can't find card in store, nothing to do here
      snackbarStore.warning("Can't find daily card info for this ID: " + cardId.value)
      router.push('/')
      return
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="collection-viewer" v-if="card">
    <daily-collection-viewer
      v-if="card.cardType === 'on_this_day' || card.cardType === 'cluster'"
      :card="card"
    />
    <location-estimatr v-else-if="card.cardType === 'estimatr'" :card="card" />
  </div>
</template>

<style scoped>
.collection-viewer {
  position: absolute;
  width: 100%;
  height: 100%;
  top: 0;
  left: 0;
  z-index: 1000;
  background-color: rgb(var(--v-theme-background));
  color: rgb(var(--v-theme-on-background));
}
</style>
