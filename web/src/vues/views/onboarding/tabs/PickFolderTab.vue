<script setup lang="ts">
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import FullFolderPicker from '@/vues/components/onboarding/FullFolderPicker.vue'

const authStore = useAuthStore()
</script>

<template>
  <section>
    <v-card class="folder-card" variant="text" rounded color="primary">
      <v-card-title class="d-flex align-center card-title">
        <v-icon icon="mdi-folder-outline" class="mr-2"></v-icon>
        Pick your user folder.
      </v-card-title>
      <v-card-text>
        <p class="text-medium-emphasis text-caption mb-3">
          Choose the folder where your
          <span :style="{ fontWeight: 700 }">({{ authStore.user?.name }})</span>
          photos and videos are stored. You can select the root of your linked media directory or a
          specific subfolder. If you invite others, their media will be organized in separate
          folders within the same root directory.
        </p>
      </v-card-text>
    </v-card>

    <full-folder-picker />
  </section>
</template>

<style scoped></style>
