<script setup lang="ts">
import { ref } from 'vue'
import SearchBar from '@/vues/components/ui/SearchBar.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import UserAvatar from '@/vues/components/ui/UserAvatar.vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { themeOptions } from '@/scripts/constants.ts'
import { caps } from '@/scripts/utils.ts'
import IngestOverlayMenu from '@/vues/components/activity/IngestOverlayMenu.vue'

const authStore = useAuthStore()
const settings = useSettingStore()
const systemStore = useSystemStore()

const menuOpen = ref(false)
const ingestMenuOpen = ref(false)

async function logout() {
  menuOpen.value = false
  await authStore.logout(false)
  location.reload()
}
</script>

<template>
  <v-app-bar density="comfortable" :height="70" class="header" color="transparent" elevation="0">
    <h1 class="appbar-title"><span>Ruurd</span> Photos</h1>
    <v-spacer />
    <search-bar v-if="authStore.isAuthenticated" />
    <v-spacer />
    <div v-if="authStore.isAuthenticated" class="header-buttons">
      <!-- Sync Menu overlay for background ingestion state (hidden when on full activity page) -->
      <v-menu
        v-if="systemStore.stats.isIngesting"
        v-model="ingestMenuOpen"
        :close-on-content-click="false"
        location="bottom end"
        offset="10"
        transition="slide-y-transition"
      >
        <template v-slot:activator="{ props }">
          <v-btn icon v-bind="props" variant="text" color="primary" class="mr-1">
            <v-icon class="spinning-sync-icon">mdi-sync</v-icon>
          </v-btn>
        </template>
        <ingest-overlay-menu @close-menu="ingestMenuOpen = false" />
      </v-menu>

      <v-btn variant="plain" rounded prepend-icon="mdi-upload"> Upload </v-btn>
      <v-menu v-model="menuOpen" :close-on-content-click="false">
        <template v-slot:activator="{ props }">
          <v-btn icon v-bind="props">
            <user-avatar
              v-if="authStore.user"
              :name="authStore.user.name"
              :avatar-id="authStore.user.avatarId"
            />
          </v-btn>
        </template>
        <div class="menu-container">
          <router-link
            @click="menuOpen = false"
            v-if="authStore.user"
            :to="`/user/${authStore.user.id}/${encodeURIComponent(authStore.user.name)}`"
          >
            <v-sheet color="surface-variant" class="pb-5" v-ripple to="/profile">
              <div class="menu-header">
                <div class="user-icon">
                  <user-avatar
                    size="50"
                    v-if="authStore.user"
                    :name="authStore.user.name"
                    :avatar-id="authStore.user.avatarId"
                  />
                </div>
                <div class="user-info">
                  <p class="user-name">{{ authStore.user.name }}</p>
                  <p class="user-email">{{ authStore.user.email }}</p>
                </div>
              </div>
            </v-sheet>
          </router-link>
          <v-list bg-color="surface-container">
            <v-list-item>
              <div class="mt-1 theme-container">
                <v-list-item-title class="theme-title">Theme</v-list-item-title>

                <v-chip-group
                  v-model="settings.themeString"
                  color="primary"
                  class="chip-group"
                  content-class="theme-item"
                  mandatory
                >
                  <v-chip
                    v-for="opt in themeOptions.slice(0, 3)"
                    :value="opt"
                    class="theme-chip"
                    variant="flat"
                    :key="opt"
                  >
                    {{ caps(opt) }}
                  </v-chip>
                </v-chip-group>
              </div>
            </v-list-item>
            <v-divider class="mb-2 mt-2" />
            <v-list-item
              v-if="authStore.user"
              prepend-icon="mdi-account-circle"
              :to="`/user/${authStore.user.id}/${encodeURIComponent(authStore.user.name)}`"
              @click="menuOpen = false"
            >
              <v-list-item-title>Profile</v-list-item-title>
            </v-list-item>
            <v-list-item prepend-icon="mdi-logout" @click="logout">
              <v-list-item-title>Sign out</v-list-item-title>
            </v-list-item>

            <v-divider class="mb-2 mt-2" />

            <v-list-item
              prepend-icon="mdi-security"
              to="/admin"
              v-if="authStore.isAdmin"
              @click="menuOpen = false"
            >
              <v-list-item-title>Admin</v-list-item-title>
            </v-list-item>
            <v-list-item prepend-icon="mdi-sync" to="/activity" @click="menuOpen = false">
              <v-list-item-title>Activity</v-list-item-title>
            </v-list-item>
            <v-list-item prepend-icon="mdi-cog" to="/settings" @click="menuOpen = false">
              <v-list-item-title>Settings</v-list-item-title>
            </v-list-item>
          </v-list>
        </div>
      </v-menu>
    </div>
    <div v-else class="header-buttons">
      <v-btn to="/login" variant="tonal" rounded prepend-icon="mdi-login" class="mr-3">
        Login
      </v-btn>
    </div>
  </v-app-bar>
</template>

<style scoped>
.appbar-title {
  font-weight: 600;
  font-size: 20px;
  margin-left: 50px;
  opacity: 0.8;
}

.appbar-title > span {
  font-weight: 400;
}

.header-buttons {
  display: flex;
  gap: 20px;
  align-items: center;
}

.menu-container {
  border-radius: 20px;
  overflow: hidden;
  user-select: none;
  box-shadow: 0 10px 20px 0 rgba(0, 0, 0, 0.3);
}

.menu-container > * {
  text-decoration: none !important;
}

.theme-container {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.theme-container :deep(.theme-item) {
  transform: translateX(4px);
}

.menu-header {
  padding: 20px;
  padding-bottom: 0;
  display: flex;
  gap: 20px;
  align-items: center;
}

.user-info p {
  margin: 0;
}

.user-name {
  font-weight: bold;
}

.user-email {
  opacity: 0.7;
}

.theme-title {
  font-size: 11px;
  text-transform: uppercase;
  font-weight: 300;
  opacity: 0.7;
  text-align: center;
}

.spinning-sync-icon {
  animation: rotation 3s infinite linear;
}

@keyframes rotation {
  from {
    transform: rotate(360deg);
  }
  to {
    transform: rotate(0deg);
  }
}
</style>
