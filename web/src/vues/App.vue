<script setup lang="ts">
import { RouterView } from 'vue-router'
import SnackbarQueue from '@/vues/components/SnackbarQueue.vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { watch } from 'vue'
import DialogQueue from '@/vues/components/DialogQueue.vue'
import { useThemeStore } from '@/scripts/stores/themeStore.ts'

const settings = useSettingStore()
const themeStore = useThemeStore()

themeStore.initThemeSync()

watch(
  () => settings.useBackdropBlur,
  () => {
    if (settings.useBackdropBlur) {
      document.body.classList.add('backdrop-blur')
    } else {
      document.body.classList.remove('backdrop-blur')
    }
  },
  { immediate: true },
)
</script>

<template>
  <v-app class="main-content">
    <RouterView />
  </v-app>

  <snackbar-queue />
  <dialog-queue />
</template>

<style scoped>
.main-content {
  width: 100%;
  height: 100vh;
  overflow-y: auto;
  user-select: none;
  background-color: #101010 !important;
}
</style>
