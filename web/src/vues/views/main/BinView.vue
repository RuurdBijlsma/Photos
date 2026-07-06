<script setup lang="ts">
import { onMounted } from 'vue'
import { useBinStore } from '@/scripts/stores/binStore.ts'
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'

const binStore = useBinStore()
const systemStore = useSystemStore()

async function emptyBin() {
  const ids = binStore.binItems.map((item) => item.id)
  if (ids.length === 0) return
  await binStore.hardDeleteItems(ids)
}

onMounted(() => {
  binStore.fetchBin()
})
</script>

<template>
  <div class="bin-container">
    <simple-timeline
      :ideal-row-height="200"
      :timeline-items="binStore.binItems"
      view-link="/bin/view/"
      :context="{ isBin: true }"
    >
      <header class="library-header">
        <div class="header-left">
          <h1>Bin</h1>
          <span class="album-count">
            {{ binStore.binItems.length }} item{{ binStore.binItems.length === 1 ? '' : 's' }}
          </span>
        </div>

        <div class="header-actions d-flex align-center">
          <v-btn
            v-if="binStore.binItems.length > 0 && systemStore.stats.allowFileDeletion"
            variant="text"
            color="error"
            prepend-icon="mdi-delete-empty"
            rounded="xl"
            class="text-none"
            @click="emptyBin"
          >
            Empty Bin
          </v-btn>
        </div>
      </header>

      <!-- Loading State -->
      <div v-if="binStore.isLoading && binStore.binItems.length === 0" class="loading-state">
        <v-progress-circular indeterminate color="primary" size="50" />
      </div>

      <!-- Empty State -->
      <div v-else-if="!binStore.isLoading && binStore.binItems.length === 0" class="empty-state">
        <v-icon icon="mdi-delete-outline" size="100" class="mb-4 opacity-20" />
        <h2>Bin is empty</h2>
        <p>
          Once you delete photos or videos, they will appear where you have the choice to
          permanently delete them.
        </p>
      </div>
    </simple-timeline>
  </div>
</template>

<style scoped>
.bin-container {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
}

.library-header {
  padding: 20px 20px 10px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left h1 {
  margin: 0;
  font-size: 2.5rem;
  font-weight: 600;
  line-height: 1.2;
}

.album-count {
  font-size: 0.9rem;
  font-weight: 400;
  color: rgb(var(--v-theme-on-surface-variant));
}

.loading-state {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 300px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 100px 0;
  text-align: center;
}

.empty-state h2 {
  opacity: 0.8;
  margin-top: 10px;
}

.empty-state p {
  opacity: 0.6;
  margin-top: 5px;
}
</style>
