<script setup lang="ts">
import { RouterView } from 'vue-router'
import SnackbarQueue from '@/vues/components/SnackbarQueue.vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { watch } from 'vue'
import DialogQueue from '@/vues/components/DialogQueue.vue'
import { useThemeStore } from '@/scripts/stores/themeStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useRegisterSW } from 'virtual:pwa-register/vue'
import MdiRefresh from '~icons/mdi/refresh'
import { useResponsive } from '@/scripts/composables/useResponsive'
import { useTheme } from 'vuetify'

const settings = useSettingStore()
const themeStore = useThemeStore()
const snackbarsStore = useSnackbarsStore()

themeStore.initThemeSync()
const { isMobile } = useResponsive()
const theme = useTheme()

// Register the Service Worker and destructure status states
const { offlineReady, needRefresh, updateServiceWorker } = useRegisterSW()

// Watch for the offline-ready event (cache initialized)
watch(offlineReady, (ready) => {
  if (ready) {
    snackbarsStore.success('App is ready to work offline.')
  }
})

watch(
  () => theme.global.name.value,
  (newTheme) => {
    let metaThemeColor = document.querySelector('meta[name="theme-color"]')
    if (!metaThemeColor) {
      metaThemeColor = document.createElement('meta')
      metaThemeColor.setAttribute('name', 'theme-color')
      document.head.appendChild(metaThemeColor)
    }
    // Set to the surface / background color of your current theme
    const color = newTheme === 'dark' ? '#101010' : '#FFF0EE'
    metaThemeColor.setAttribute('content', color)
  },
  { immediate: true },
)

// Watch for update availability (new files detected on server)
watch(needRefresh, (refresh) => {
  if (refresh) {
    // Using the direct enqueue function to set a timeout of 0,
    // keeping the update notification persistent until resolved.
    snackbarsStore.enqueue({
      message: 'New version available. Reload to update.',
      color: 'info',
      icon: MdiRefresh,
      timeout: 0,
      action: {
        label: 'Reload',
        onClick: () => {
          updateServiceWorker(true)
        },
        hideOnClick: true,
      },
    })
  }
})

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
  <v-app class="main-content" :class="{ 'is-mobile': isMobile, 'is-desktop': !isMobile }">
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
