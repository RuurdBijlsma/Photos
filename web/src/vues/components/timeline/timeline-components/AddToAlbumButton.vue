<script setup lang="ts">
import { ref } from 'vue'
import AddToAlbumCard from '@/vues/components/timeline/timeline-components/AddToAlbumCard.vue'

withDefaults(
  defineProps<{
    excludeAlbumIds?: string[]
    idsToAdd: string[]
  }>(),
  {
    excludeAlbumIds: () => [],
  },
)

const show = ref(false)
</script>

<template>
  <v-menu v-model="show" :close-on-content-click="false" location="top center" :offset="[25, 0]">
    <template v-slot:activator="{ props }">
      <v-btn
        v-bind="props"
        :icon="show ? 'mdi-chevron-up' : 'mdi-plus'"
        variant="plain"
        density="compact"
        :disabled="show"
        v-tooltip:top="'Add to album'"
      />
    </template>

    <add-to-album-card :ids-to-add="idsToAdd" :exclude-album-ids="excludeAlbumIds" />
  </v-menu>
</template>

<style scoped></style>
