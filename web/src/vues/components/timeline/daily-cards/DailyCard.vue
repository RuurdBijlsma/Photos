<script setup lang="ts">
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import type { DailyCardResponse } from '@/scripts/types/api/dailyCards.ts'
import { useDailyCardStore } from '@/scripts/stores/timeline/dailyCardStore.ts'

const cardStore = useDailyCardStore()

defineProps<{
  card: DailyCardResponse
  width: number
}>()

function isGame(cardType: string) {
  return ['estimatr'].includes(cardType)
}
</script>

<template>
  <v-theme-provider theme="dark" with-background class="theme-prov">
    <router-link :to="`/daily/${card.id}`" class="daily-card" v-ripple>
      <thumbnail-img
        v-if="card.thumbnailMediaItemId"
        :media-item-id="card.thumbnailMediaItemId"
        class="card-thumb"
        cover
      />
      <div class="card-thumb fake-thumb" v-else>
        <v-icon icon="mdi-image-area" color="primary" size="150"></v-icon>
      </div>
      <div class="card-content">
        <v-spacer />
        <div class="card-bottom">
          <div class="card-info">
            <h2>{{ card.title }}</h2>
            <p>{{ card.subtitle }}</p>
          </div>
          <div class="card-icon-container">
            <v-icon
              size="40"
              :icon="cardStore.completedCards.includes(card.id) ? `mdi-check` : `mdi-controller`"
              color="on-surface-variant"
              v-if="isGame(card.cardType)"
            />
          </div>
        </div>
      </div>
    </router-link>
  </v-theme-provider>
</template>

<style scoped>
.theme-prov {
  background-color: transparent;
}

.daily-card {
  display: block;
  height: 100%;
  width: calc(v-bind(width) * 1px);
  border-radius: 40px;
  flex-shrink: 0;
  flex-grow: 0;
  position: relative;
  cursor: pointer;
  transition: filter 0.2s;
  color: inherit;
}

.daily-card:hover {
  filter: brightness(120%);
}

.daily-card * {
  pointer-events: none;
}

.card-content {
  display: flex;
  flex-direction: column;
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0) 30%, rgba(0, 0, 0, 0.6) 100%);
  border-radius: 40px;
  overflow: hidden;
}

.card-bottom {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-icon-container {
  padding-right: 30px;
  opacity: 0.7;
}

.card-info {
  padding: 15px 30px;
}

.card-thumb {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  border-radius: 40px;
  overflow: hidden;
}

.fake-thumb {
  display: flex;
  place-items: center;
  place-content: center;
  opacity: 0.3;
}

.daily-card h2 {
  margin: 0;
  font-weight: 600;
}

.daily-card p {
  margin: 0;
}
</style>
