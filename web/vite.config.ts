import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import vueDevTools from 'vite-plugin-vue-devtools'
import vuetify from 'vite-plugin-vuetify'
import { VitePWA } from 'vite-plugin-pwa'

const repoName = 'Photos'

// https://vitejs.dev/config/
export default defineConfig({
  // Conditionally set the base path for GitHub Pages deployment
  base: process.env.GITHUB_PAGES ? `/${repoName}/` : '/',
  server: {
    proxy: {
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
    },
  },
  plugins: [
    vue(),
    vueJsx(),
    vueDevTools(),
    vuetify({ autoImport: { labs: true } }),
    Icons({
      compiler: 'vue3',
    }),
    VitePWA({
      registerType: 'prompt', // Prompt users before updating (allows showing a reload notification)
      injectRegister: 'auto',
      includeAssets: ['favicon.ico', 'apple-touch-icon.png', 'favicon.svg'],
      workbox: {
        globPatterns: ['**/*.{js,css,html,ico,png,svg}'], // Files to cache for offline use
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
