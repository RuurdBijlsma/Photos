<script setup lang="ts">
import { useBackgroundStore } from '@/scripts/stores/backgroundStore'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import NavDrawer from '@/vues/components/layout/NavDrawer.vue'
import AppBar from '@/vues/components/layout/AppBar.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import BottomNav from '@/vues/components/layout/BottomNav.vue'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'

// Instantiate stores
const backgroundStore = useBackgroundStore()
const settings = useSettingStore()
const authStore = useAuthStore()
const responsive = useResponsive()

// Initialize the stores.
backgroundStore.initialize()
</script>

<template>
  <div class="blurred-background">
    <div
      class="background-image"
      :style="{
        backgroundImage: settings.useImageBackground ? `url(${backgroundStore.backgroundUrl})` : '',
        filter: settings.useImageBackground
          ? 'saturate(150%) brightness(70%) blur(25px) contrast(100%)'
          : '',
      }"
    ></div>
    <div v-if="settings.useImageBackground" class="gradient-overlay"></div>
  </div>

  <v-layout>
    <app-bar />

    <template v-if="authStore.isAuthenticated">
      <bottom-nav v-if="responsive.isMobile.value" />
      <nav-drawer v-else />
    </template>
    <template v-else>
      <v-navigation-drawer :width="40" floating color="transparent"></v-navigation-drawer>
    </template>

    <v-main class="layout-body">
      <router-view class="router-view" />
    </v-main>
  </v-layout>
</template>

<style scoped>
.blurred-background {
  position: fixed;
  width: 100%;
  height: 100%;
  background-color: #363654;
  z-index: 0;
}

.blurred-background > div {
  position: fixed;
  width: 100%;
  height: 100%;
}

.background-image {
  z-index: 0;
  background-size: cover;
  background-position: center;
  width: 100%;
  height: 100%;
  background-color: rgb(var(--v-theme-surface-container-high));
}

.gradient-overlay {
  position: fixed;
  width: 100%;
  height: 100%;
  background-image: linear-gradient(
    180deg,
    rgba(var(--v-theme-background), 0.95) 0%,
    rgba(var(--v-theme-background), 0.4) 100%
  );
  z-index: 1;
}

.layout-body {
  display: flex;
  width: 100vw;
  height: 100%;
}
</style>
