import './assets/main.css'

import { createApp } from 'vue'

import App from './vues/App.vue'
import router, { registerNavigationGuard } from '@/scripts/plugins/router'
import { vuetify } from '@/scripts/plugins/vuetify'
import { pinia } from '@/scripts/plugins/pinia'

const app = createApp(App)

app.use(pinia)
registerNavigationGuard()
app.use(router)
app.use(vuetify)
app.mount('#app')
