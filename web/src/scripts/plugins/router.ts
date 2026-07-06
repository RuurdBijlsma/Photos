import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '@/vues/layouts/MainLayout.vue'
import TimelineView from '@/vues/views/main/TimelineView.vue'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useTimelineStore } from '@/scripts/stores/timeline/timelineStore.ts'

const ViewPhoto = () => import('@/vues/views/main/ViewPhoto.vue')

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'home',
      component: MainLayout,
      meta: { requiresAuth: true },
      children: [
        {
          path: '',
          name: 'timeline',
          component: TimelineView,
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-timeline',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'daily/:cardId',
          name: 'daily-card-viewer',
          component: () => import('@/vues/components/timeline/daily-cards/DailyViewer.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-daily',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'people',
          name: 'people',
          component: () => import('@/vues/views/library/PeopleLibrary.vue'),
        },
        {
          path: 'person/:personId',
          name: 'person-view',
          component: () => import('@/vues/views/library/PersonView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-person',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'cameras',
          name: 'cameras',
          component: () => import('@/vues/views/library/CamerasLibrary.vue'),
        },
        {
          path: 'camera/:cameraMake/:cameraModel',
          name: 'camera-view',
          component: () => import('@/vues/views/library/CameraView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-camera',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'explore',
          name: 'explore',
          component: () => import('@/vues/views/main/ExploreView.vue'),
        },
        {
          path: 'bin',
          name: 'bin',
          component: () => import('@/vues/views/main/BinView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-bin',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'storage',
          name: 'storage',
          component: () => import('@/vues/views/main/StorageView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-storage',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'storage/review',
          name: 'storage-review',
          component: () => import('@/vues/views/main/StorageReviewView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-storage-review',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'storage/blurry',
          name: 'storage-blurry',
          component: () => import('@/vues/views/main/StorageReviewView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-storage-blurry',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'map',
          name: 'map',
          component: () => import('@/vues/views/main/MapView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-map',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'albums',
          name: 'albums',
          component: () => import('@/vues/views/library/AlbumsLibrary.vue'),
        },
        {
          path: 'settings',
          name: 'settings',
          component: () => import('@/vues/views/main/SettingsView.vue'),
        },
        {
          path: 'activity',
          name: 'activity',
          component: () => import('@/vues/views/main/ActivityView.vue'),
        },
        {
          path: 'admin',
          name: 'admin',
          meta: { requiresAdmin: true },
          component: () => import('@/vues/views/main/AdminView.vue'),
        },
        {
          path: 'user/:userId/:name',
          name: 'profile',
          component: () => import('@/vues/views/main/ProfileView.vue'),
        },
        {
          path: 'album/:albumId',
          name: 'album-view',
          meta: { requiresAuth: false },
          component: () => import('@/vues/views/library/AlbumView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-album',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'search',
          name: 'search',
          component: () => import('@/vues/views/main/SearchView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-search',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: '/import-album/:token',
          name: 'import-album',
          meta: { requiresAuth: true },
          component: () => import('@/vues/views/main/ImportAlbumView.vue'),
        },
      ],
    },
    {
      path: '/login',
      name: 'login',
      meta: { guest: true },
      component: () => import('@/vues/views/auth/LoginView.vue'),
    },
    {
      path: '/register',
      name: 'register',
      meta: { guest: true },
      component: () => import('@/vues/views/auth/RegisterView.vue'),
    },
    {
      path: '/onboarding',
      name: 'onboarding',
      meta: { requiresAuth: true, requiresAdmin: true },
      component: () => import('@/vues/views/onboarding/OnboardingView.vue'),
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'not-found',
      component: () => import('@/vues/views/NotFoundView.vue'),
    },
  ],
})

let userRefreshed = false
let onAuthHandled = false

export function registerNavigationGuard() {
  const snackbarsStore = useSnackbarsStore()

  router.beforeEach(async (to) => {
    const authStore = useAuthStore()

    // --- Authentication Initialization ---
    // If we have a token but no user data, attempt to fetch it. This is crucial for page reloads.
    if (authStore.accessToken && !authStore.user) {
      try {
        await authStore.fetchCurrentUser()
      } catch (error) {
        // If the token is invalid, the fetch will fail. Log the user out completely.
        console.error('Session restore failed:', error)
        await authStore.logout()
        // No need to proceed further, just go to login.
        console.warn('[router -> 1] redirect to /login')
        return { name: 'login' }
      }
    } else if (authStore.accessToken && authStore.user && !userRefreshed) {
      userRefreshed = true
      requestIdleCallback(() => authStore.fetchCurrentUser())
    }

    // --- Get Fresh Auth State ---
    const isAuthenticated = authStore.isAuthenticated
    const isAdmin = authStore.isAdmin

    if (isAuthenticated && !onAuthHandled) {
      onAuthHandled = true
      requestIdleCallback(() => authStore.onAuthenticated())
    } else if (!isAuthenticated) {
      onAuthHandled = false
    }

    // --- "Onboarding Needed" Redirect Logic ---
    const needsOnboarding =
      isAdmin && (authStore.user?.mediaFolder === null || authStore.user?.mediaFolder === undefined)
    if (needsOnboarding && to.name !== 'onboarding') {
      return { name: 'onboarding' }
    }
    // If onboarding is needed, and we are already going to the onboarding page, allow it.
    if (needsOnboarding && to.name === 'onboarding') {
      return true
    }

    // --- Admin Route Logic ---
    if (to.meta.requiresAdmin) {
      if (isAuthenticated && isAdmin) {
        return true
      } else {
        snackbarsStore.error("You don't have permission to access this page.")
        if (isAuthenticated) {
          return { name: 'timeline' }
        } else {
          console.warn('[router -> 2] redirect to /login')
          return { name: 'login' }
        }
      }
    }

    // --- Authenticated Route Logic ---
    if (to.meta.requiresAuth) {
      if (isAuthenticated) {
        return true
      } else {
        console.warn('[router meta requires auth] redirect to /login')
        return { name: 'login' }
      }
    }

    // --- Guest Route Logic (for pages like Login and Register) ---
    if (to.meta.guest) {
      if (isAuthenticated) {
        return { name: 'timeline' }
      } else {
        return true
      }
    }

    // Slightly faster load time for timeline
    if (to.name === 'timeline') {
      const timelineStore = useTimelineStore()
      if (!timelineStore.isInitialized) timelineStore.initialize().then()
    }

    // --- Fallback for public routes ---
    return true
  })
}

export default router
