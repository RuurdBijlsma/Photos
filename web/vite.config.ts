import { fileURLToPath, URL } from 'node:url'

import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import vueDevTools from 'vite-plugin-vue-devtools'
import vuetify from 'vite-plugin-vuetify'
import { VitePWA } from 'vite-plugin-pwa'
import Icons from 'unplugin-icons/vite'

const repoName = 'Photos'

const BACKEND_DELAY_MS = 1500

// todo: remove delay
function throttleBackendPlugin(delayMs: number): Plugin {
  return {
    name: 'throttle-backend-requests',
    configureServer(server) {
      if (delayMs <= 0) return

      server.middlewares.use((req, _res, next) => {
        // Only throttle proxied backend requests
        const isBackend =
          req.url?.startsWith('/api') ||
          req.url?.startsWith('/thumbnails') ||
          req.url?.startsWith('/hosted')

        if (isBackend) {
          setTimeout(next, delayMs)
        } else {
          next()
        }
      })
    },
  }
}

const proxyConfig = {
  '/api': {
    target: 'http://localhost:5272',
    changeOrigin: true,
    ws: true,
  },
  '/thumbnails': {
    target: 'http://localhost:5272',
    changeOrigin: true,
  },
  '/hosted': {
    target: 'http://localhost:5272',
    changeOrigin: true,
  },
}

// https://vitejs.dev/config/
export default defineConfig({
  base: process.env.GITHUB_PAGES ? `/${repoName}/` : '/',
  server: {
    proxy: proxyConfig,
  },
  preview: {
    port: 4173,
    proxy: proxyConfig,
  },
  plugins: [
    vue(),
    vueJsx(),
    vueDevTools(),
    throttleBackendPlugin(BACKEND_DELAY_MS),
    vuetify({ autoImport: { labs: true } }),
    Icons({
      compiler: 'vue3',
    }),
    VitePWA({
      registerType: 'prompt',
      injectRegister: 'auto',
      includeAssets: ['favicon.ico', 'apple-touch-icon.png', 'favicon.svg'],
      workbox: {
        globPatterns: ['**/*.{js,css,html,ico,png,svg}'],
      },
      manifest: {
        name: 'Ruurd Photos',
        short_name: 'Photos',
        description: 'Manage your photos and videos',
        theme_color: '#101010',
        background_color: '#101010',
        display: 'standalone',
        start_url: '.',
        icons: [
          {
            src: 'favicon-96x96.png',
            sizes: '96x96',
            type: 'image/png',
          },
          {
            src: 'web-app-manifest-192x192.png',
            sizes: '192x192',
            type: 'image/png',
          },
          {
            src: 'web-app-manifest-512x512.png',
            sizes: '512x512',
            type: 'image/png',
          },
          {
            src: 'web-app-manifest-512x512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
    }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
})
