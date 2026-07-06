import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import vueDevTools from 'vite-plugin-vue-devtools'
import vuetify from 'vite-plugin-vuetify'

const repoName = 'photos-frontend'

// https://vitejs.dev/config/
export default defineConfig({
  // Conditionally set the base path for GitHub Pages deployment
  base: process.env.GITHUB_PAGES ? `/${repoName}/` : '/',
  plugins: [vue(), vueJsx(), vueDevTools(), vuetify({ autoImport: { labs: true } })],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
})
