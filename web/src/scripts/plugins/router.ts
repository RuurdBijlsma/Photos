import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '@/vues/layouts/MainLayout.vue'
import TimelineView from '@/vues/views/main/TimelineView.vue'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useTimelineStore } from '@/scripts/stores/timeline/timelineStore.ts'
import { useTitleStore } from '@/scripts/stores/titleStore.ts'

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
          meta: { title: 'Photos' },
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-timeline',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'share/:viewerType/:mediaId',
          name: 'share',
          meta: { requiresAuth: false, title: 'Shared Media' },
          component: () => import('@/vues/views/main/ShareView.vue'),
        },
        {
          path: 'daily/:cardId',
          name: 'daily-card-viewer',
          meta: { title: 'Daily' },
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
          meta: { title: 'People' },
          component: () => import('@/vues/views/library/PeopleLibrary.vue'),
        },
        {
          path: 'person/:personId',
          name: 'person-view',
          meta: { fallbackTitle: 'Person' },
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
          meta: { title: 'Cameras' },
          component: () => import('@/vues/views/library/CamerasLibrary.vue'),
        },
        {
          path: 'camera/:cameraMake/:cameraModel',
          name: 'camera-view',
          meta: { fallbackTitle: 'Camera' },
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
          meta: { title: 'Explore' },
          component: () => import('@/vues/views/main/ExploreView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-explore',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'explore/location/:locationId',
          name: 'explore-location-view',
          meta: { fallbackTitle: 'Place' },
          component: () => import('@/vues/views/main/LocationView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-explore-location',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'bin',
          name: 'bin',
          meta: { title: 'Bin' },
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
          meta: { title: 'Storage' },
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
          meta: { title: 'Storage Review' },
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
          meta: { title: 'Blurry Photos' },
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
          meta: { title: 'Map' },
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
          meta: { title: 'Albums' },
          component: () => import('@/vues/views/library/AlbumsLibrary.vue'),
        },
        {
          path: 'settings',
          name: 'settings',
          meta: { title: 'Settings' },
          component: () => import('@/vues/views/main/SettingsView.vue'),
          children: [
            {
              path: 'view/:mediaId',
              name: 'view-photo-settings',
              component: ViewPhoto,
            },
          ],
        },
        {
          path: 'activity',
          name: 'activity',
          meta: { title: 'Activity' },
          component: () => import('@/vues/views/main/ActivityView.vue'),
        },
        {
          path: 'admin',
          name: 'admin',
          meta: { requiresAdmin: true, title: 'Admin' },
          component: () => import('@/vues/views/main/AdminView.vue'),
        },
        {
          path: 'user/:userId/:name',
          name: 'profile',
          meta: { fallbackTitle: 'Profile' },
          component: () => import('@/vues/views/main/ProfileView.vue'),
        },
        {
          path: 'album/:albumId',
          name: 'album-view',
          meta: { requiresAuth: false, fallbackTitle: 'Album' },
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
          meta: { fallbackTitle: 'Search' },
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
          meta: { requiresAuth: true, title: 'Import Album' },
          component: () => import('@/vues/views/main/ImportAlbumView.vue'),
        },
      ],
    },
    {
      path: '/login',
      name: 'login',
      meta: { guest: true, title: 'Login' },
      component: () => import('@/vues/views/auth/LoginView.vue'),
    },
    {
      path: '/register',
      name: 'register',
      meta: { guest: true, title: 'Register' },
      component: () => import('@/vues/views/auth/RegisterView.vue'),
    },
    {
      path: '/onboarding',
      name: 'onboarding',
      meta: { requiresAuth: true, requiresAdmin: true, title: 'Setup' },
      component: () => import('@/vues/views/onboarding/OnboardingView.vue'),
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'not-found',
      meta: { title: 'Not found' },
      component: () => import('@/vues/views/NotFoundView.vue'),
    },
  ],
})

router.afterEach((to) => {
  const titleStore = useTitleStore()

  const isViewMediaRoute = to.name?.toString().startsWith('view-photo-')

  if (!isViewMediaRoute) {
    titleStore.setDetailTitle(null)
  }

  const matchedRoute =
    isViewMediaRoute && to.matched.length > 1
      ? to.matched[to.matched.length - 2]
      : to.matched[to.matched.length - 1]

  const titleMeta = matchedRoute?.meta.title as string | undefined
  const fallbackMeta = matchedRoute?.meta.fallbackTitle as string | undefined

  if (titleMeta) {
    titleStore.setPageTitle(titleMeta)
  } else if (!fallbackMeta) {
    titleStore.setPageTitle(null)
  }
})

let sessionChecked = false
let onAuthHandled = false

export function registerNavigationGuard() {
  const snackbarsStore = useSnackbarsStore()

  router.beforeEach(async (to) => {
    const authStore = useAuthStore()

    if (!sessionChecked) {
      sessionChecked = true
      try {
        await authStore.fetchCurrentUser()
      } catch (error) {
        console.warn('Session restore failed or no active session:', error)
        await authStore.logout(false)
      }
    }

    const isAuthenticated = authStore.isAuthenticated
    const isAdmin = authStore.isAdmin

    if (isAuthenticated && !onAuthHandled) {
      onAuthHandled = true
      requestIdleCallback(() => authStore.onAuthenticated())
    } else if (!isAuthenticated) {
      onAuthHandled = false
    }

    const needsOnboarding =
      isAdmin && (authStore.user?.mediaFolder === null || authStore.user?.mediaFolder === undefined)
    if (needsOnboarding && to.name !== 'onboarding') {
      return { name: 'onboarding' }
    }
    if (needsOnboarding && to.name === 'onboarding') {
      return true
    }

    if (to.meta.requiresAdmin) {
      if (isAuthenticated && isAdmin) {
        return true
      } else {
        snackbarsStore.error("You don't have permission to access this page.")
        if (isAuthenticated) {
          return { name: 'timeline' }
        } else {
          return { name: 'login' }
        }
      }
    }

    if (to.meta.requiresAuth) {
      if (isAuthenticated) {
        return true
      } else {
        return { name: 'login' }
      }
    }

    if (to.meta.guest) {
      if (isAuthenticated) {
        return { name: 'timeline' }
      } else {
        return true
      }
    }

    if (to.name === 'timeline') {
      const timelineStore = useTimelineStore()
      if (!timelineStore.isInitialized) timelineStore.initialize().then()
    }

    return true
  })
}

export default router
