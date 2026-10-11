<script setup lang="ts">
import MdiImageAlbum from '~icons/mdi/image-album'

defineProps<{
  isImporting?: boolean
}>()
</script>

<template>
  <div class="empty-album">
    <v-icon color="on-surface-variant" size="200" :icon="MdiImageAlbum" />
    <h2>This album is empty</h2>
    <template v-if="isImporting">
      <p>Importing media from remote server...</p>
      <v-progress-circular class="mt-5" indeterminate size="50" />
    </template>
  </div>
</template>

<style scoped>
.empty-album {
  height: 700px;
  width: 100%;
  display: flex;
  place-items: center;
  place-content: center;
  flex-direction: column;
  color: rgb(var(--v-theme-on-surface-variant));
}
</style>
