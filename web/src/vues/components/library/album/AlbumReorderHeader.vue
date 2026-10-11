<script setup lang="ts">
import MdiCheck from '~icons/mdi/check'
import MdiPencil from '~icons/mdi/pencil'
import MdiSort from '~icons/mdi/sort'
import MdiSortCalendarAscending from '~icons/mdi/sort-calendar-ascending'
import MdiSortCalendarDescending from '~icons/mdi/sort-calendar-descending'
import MdiSortClockAscendingOutline from '~icons/mdi/sort-clock-ascending-outline'
import MdiSortClockDescendingOutline from '~icons/mdi/sort-clock-descending-outline'
import type { AlbumInfo } from '@/scripts/types/generated/timeline.ts'
import type { AlbumSort } from '@/scripts/types/api/album.ts'

defineProps<{
  album: AlbumInfo | null
}>()

const emit = defineEmits<{
  (e: 'previewSort', mode: AlbumSort): void
  (e: 'cancel'): void
  (e: 'save'): void
}>()
</script>

<template>
  <div class="album-reorder-header">
    <div class="reorder-header-title">
      <v-icon :icon="MdiPencil" class="mr-5" size="25" />
      <p>Edit album order</p>
    </div>
    <div>
      <v-menu location="bottom start">
        <template v-slot:activator="{ props }">
          <v-btn
            v-bind="props"
            class="mr-5"
            variant="text"
            rounded
            color="primary"
            :prepend-icon="MdiSort"
          >
            Sort photos
          </v-btn>
        </template>
        <v-list density="compact" bg-color="surface-container-high">
          <v-list-item
            v-if="album"
            :active="album.sortMode == 'DateDesc'"
            @click="emit('previewSort', 'DateDesc')"
            :prepend-icon="MdiSortCalendarDescending"
          >
            <v-list-item-title>Newest first</v-list-item-title>
          </v-list-item>
          <v-list-item
            v-if="album"
            :active="album.sortMode == 'DateAsc'"
            @click="emit('previewSort', 'DateAsc')"
            :prepend-icon="MdiSortCalendarAscending"
          >
            <v-list-item-title>Oldest first</v-list-item-title>
          </v-list-item>
          <v-divider />
          <v-list-item
            @click="emit('previewSort', 'AddedDesc')"
            :prepend-icon="MdiSortClockDescendingOutline"
          >
            <v-list-item-title>Recently added</v-list-item-title>
          </v-list-item>
          <v-list-item
            @click="emit('previewSort', 'AddedAsc')"
            :prepend-icon="MdiSortClockAscendingOutline"
          >
            <v-list-item-title>Oldest added</v-list-item-title>
          </v-list-item>
        </v-list>
      </v-menu>
      <v-btn variant="text" rounded class="mr-2" @click="emit('cancel')">Cancel</v-btn>
      <v-btn variant="tonal" color="primary" rounded @click="emit('save')" :prepend-icon="MdiCheck">
        Save
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
.album-reorder-header {
  display: flex;
  padding: 20px 25px;
  border-radius: 50px;
  width: calc(100% - 50px);
  margin-right: 50px;
  margin-top: 10px;
  margin-bottom: 10px;
  background-color: rgb(var(--v-theme-background));
  box-shadow: inset 0 0 0 1px rgba(var(--v-theme-primary), 0.1);
  justify-content: space-between;
  align-items: center;
}

.reorder-header-title {
  display: flex;
  align-items: center;
}

.reorder-header-title p {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}
</style>
