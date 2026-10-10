<script setup lang="ts">
import MdiAccountCircle from '~icons/mdi/account-circle'
import MdiAlertCircle from '~icons/mdi/alert-circle'
import MdiCog from '~icons/mdi/cog'
import MdiLogin from '~icons/mdi/login'
import MdiLogout from '~icons/mdi/logout'
import MdiSecurity from '~icons/mdi/security'
import MdiSync from '~icons/mdi/sync'
import MdiUpload from '~icons/mdi/upload'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import SearchBar from '@/vues/components/ui/SearchBar.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import UserAvatar from '@/vues/components/ui/UserAvatar.vue'
import { useSettingStore } from '@/scripts/stores/settingsStore.ts'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { themeOptions } from '@/scripts/constants.ts'
import { caps } from '@/scripts/utils.ts'
import IngestOverlayMenu from '@/vues/components/activity/IngestOverlayMenu.vue'
import transLogo from '@/assets/img/logo/transparent/192.png'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'
import { useRoute } from 'vue-router'
import { useTimelineStore } from '@/scripts/stores/timeline/timelineStore.ts'
import { useEventListener, useStorage, useThrottleFn } from '@vueuse/core'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useLayoutStore } from '@/scripts/stores/layoutStore.ts'

const authStore = useAuthStore()
const settings = useSettingStore()
const systemStore = useSystemStore()
const timelineStore = useTimelineStore()
const snackbarStore = useSnackbarsStore()
const layoutStore = useLayoutStore()
const responsive = useResponsive()
const route = useRoute()

const menuOpen = ref(false)
const ingestMenuOpen = ref(false)
const logoAngle = ref(0)
const isClicker = useStorage('clickerUnlocked', false)
let velocity = 0
let lastTime = 0
let animFrameId: number | null = null

const isSearch = computed(() => route.name === 'search')
const showSearchBar = computed(
  () => authStore.isAuthenticated && (isSearch.value || !responsive.isMobile.value),
)

const mediaFolderAvailable = computed(() => systemStore.stats.mediaFolderAvailable !== false)
const showIngestMenu = computed(() => systemStore.stats.isIngesting || !mediaFolderAvailable.value)

function updateSpin(now: number) {
  const dt = Math.min(now - lastTime, 64)
  lastTime = now

  logoAngle.value += velocity * dt
  // Friction decay (frame-rate independent)
  velocity *= Math.pow(0.96, dt / 16.67)

  // Once velocity is low enough, gently snap to the nearest full 360-degree rotation
  if (velocity < 0.05) {
    const target = Math.round(logoAngle.value / 360) * 360
    const diff = target - logoAngle.value

    if (Math.abs(diff) < 0.5) {
      logoAngle.value = 0
      velocity = 0
      animFrameId = null
      return
    }

    logoAngle.value += diff * Math.min(1, (dt / 16.67) * 0.15)
  }

  animFrameId = requestAnimationFrame(updateSpin)
}

const handleScrollToTop = useThrottleFn(scrollTimelineToTop, 1000, false, true)
function scrollTimelineToTop() {
  timelineStore.scrollToTop()
}

// --- Dynamic Scroll Target & Smooth Mobile Hide ---
const targetElement = ref<HTMLElement | Window>(window)

let lastScrollY = 0
let scrollAnchorY = 0
let lastDirection: 'up' | 'down' | null = null

const SCROLL_TOP_BUFFER = 300 // Keep visible within 300px from top
const HIDE_DELTA = 100 // Must scroll down at least 60px to hide
const SHOW_DELTA = 20 // Must scroll up at least 20px to show

// Dynamically track layoutStore.scrollTarget
watch(
  () => layoutStore.scrollTarget,
  async (newId) => {
    // Reset state on target change (route changes, container unmounts)
    lastScrollY = 0
    scrollAnchorY = 0
    lastDirection = null
    layoutStore.isAppBarVisible = true

    if (!newId) {
      targetElement.value = window
      return
    }

    await nextTick()
    const el = document.getElementById(newId)
    targetElement.value = el ?? window
  },
  { immediate: true },
)

// Ensure AppBar stays visible on desktop
watch(
  responsive.isMobile,
  (isMobile) => {
    if (!isMobile) {
      layoutStore.isAppBarVisible = true
    }
  },
  { immediate: true },
)

function getScrollTop(): number {
  if (targetElement.value instanceof Window) {
    return window.scrollY || document.documentElement.scrollTop || 0
  }
  return (targetElement.value as HTMLElement).scrollTop || 0
}

function handleScroll() {
  if (!responsive.isMobile.value) {
    layoutStore.isAppBarVisible = true
    return
  }

  const currentY = getScrollTop()

  // Always show near the top of the container
  if (currentY <= SCROLL_TOP_BUFFER) {
    layoutStore.isAppBarVisible = true
    lastScrollY = currentY
    scrollAnchorY = currentY
    lastDirection = null
    return
  }

  const delta = currentY - lastScrollY
  if (Math.abs(delta) < 1) return

  const currentDirection: 'up' | 'down' = delta > 0 ? 'down' : 'up'

  // Reset anchor whenever scroll direction reverses
  if (currentDirection !== lastDirection) {
    scrollAnchorY = lastScrollY
    lastDirection = currentDirection
  }

  if (currentDirection === 'down') {
    // Only hide if scrolled down past the anchor by at least HIDE_DELTA (60px)
    if (currentY - scrollAnchorY > HIDE_DELTA) {
      layoutStore.isAppBarVisible = false
    }
  } else {
    // Reveal if scrolled up by at least SHOW_DELTA (20px)
    if (scrollAnchorY - currentY > SHOW_DELTA) {
      layoutStore.isAppBarVisible = true
    }
  }

  lastScrollY = currentY
}

// VueUse automatically re-binds to targetElement when its value changes
useEventListener(targetElement, 'scroll', handleScroll, { passive: true })

let clickCounter = 0
const clickTarget = 100

function onLogoClick() {
  clickCounter += 1
  if (clickCounter > clickTarget - 4 && clickCounter < clickTarget)
    snackbarStore.info(`You are ${clickTarget - clickCounter} clicks away...`)
  if (clickCounter === clickTarget) {
    isClicker.value = true
    snackbarStore.info(`You are now a clicker!`)
  }
  if (!isClicker.value) return
  // Add impulse on every click
  velocity = velocity + 0.9

  if (animFrameId === null) {
    lastTime = performance.now()
    animFrameId = requestAnimationFrame(updateSpin)
  }
  handleScrollToTop()
}

onBeforeUnmount(() => {
  if (animFrameId !== null) {
    cancelAnimationFrame(animFrameId)
  }
})

async function logout() {
  menuOpen.value = false
  await authStore.logout(false)
  location.reload()
}
</script>

<template>
  <v-app-bar
    density="comfortable"
    :height="70"
    class="header"
    color="transparent"
    elevation="0"
    v-model="layoutStore.isAppBarVisible"
  >
    <!--    Mobile     -->
    <template v-if="responsive.isMobile.value">
      <img
        class="appbar-logo"
        :style="{ transform: `scale(0.5) rotate(${logoAngle}deg)` }"
        :src="transLogo"
        v-if="!isSearch"
        alt="app logo"
        @click="onLogoClick"
      />
      <v-spacer />
      <search-bar v-if="showSearchBar" />
    </template>
    <!--    Desktop    -->
    <template v-else>
      <h1 class="appbar-title"><span>Ruurd</span> Photos</h1>
      <v-spacer />
      <template v-if="authStore.isAuthenticated">
        <search-bar />
        <v-spacer />
        <v-menu
          v-if="showIngestMenu"
          v-model="ingestMenuOpen"
          :close-on-content-click="false"
          location="bottom end"
          offset="10"
          transition="slide-y-transition"
        >
          <template v-slot:activator="{ props }">
            <v-btn
              icon
              v-bind="props"
              variant="text"
              :color="mediaFolderAvailable ? 'primary' : 'error'"
              class="mr-1"
            >
              <v-icon
                :class="{
                  'spinning-sync-icon': mediaFolderAvailable && systemStore.stats.isIngesting,
                }"
                :icon="mediaFolderAvailable ? MdiSync : MdiAlertCircle"
              />
            </v-btn>
          </template>
          <ingest-overlay-menu @close-menu="ingestMenuOpen = false" />
        </v-menu>
        <v-btn variant="plain" rounded :prepend-icon="MdiUpload" to="/activity"> Upload </v-btn>
      </template>
    </template>
    <div v-if="authStore.isAuthenticated" class="header-buttons">
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
              :prepend-icon="MdiAccountCircle"
              :to="`/user/${authStore.user.id}/${encodeURIComponent(authStore.user.name)}`"
              @click="menuOpen = false"
            >
              <v-list-item-title>Profile</v-list-item-title>
            </v-list-item>
            <v-list-item :prepend-icon="MdiLogout" @click="logout">
              <v-list-item-title>Sign out</v-list-item-title>
            </v-list-item>

            <v-divider class="mb-2 mt-2" />

            <v-list-item
              :prepend-icon="MdiSecurity"
              to="/admin"
              v-if="authStore.isAdmin"
              @click="menuOpen = false"
            >
              <v-list-item-title>Admin</v-list-item-title>
            </v-list-item>
            <v-list-item :prepend-icon="MdiSync" to="/activity" @click="menuOpen = false">
              <v-list-item-title>Activity</v-list-item-title>
            </v-list-item>
            <v-list-item :prepend-icon="MdiCog" to="/settings" @click="menuOpen = false">
              <v-list-item-title>Settings</v-list-item-title>
            </v-list-item>
          </v-list>
        </div>
      </v-menu>
    </div>
    <div v-else class="header-buttons">
      <v-btn to="/login" variant="tonal" rounded :prepend-icon="MdiLogin" class="mr-3">
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

.appbar-logo {
  transform: scale(0.5);
  width: 80px;
  flex-grow: 0;
  cursor: pointer;
  user-select: none;
  -webkit-user-drag: none;
  will-change: transform;
}

.appbar-title > span {
  font-weight: 400;
}

.header-buttons {
  display: flex;
  gap: 20px;
  align-items: center;
}

.header-buttons {
  margin-right: 10px;
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

.chip-group {
  overflow-x: hidden;
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
