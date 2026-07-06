<script setup lang="ts">
import { onMounted, type Ref, ref } from 'vue'
import type { VForm } from 'vuetify/components'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useRouter } from 'vue-router'
import { isAxiosError } from 'axios'
import FocusLayout from '@/vues/layouts/FocusLayout.vue'
import AppAlert from '@/vues/components/ui/AppAlert.vue'
import { useStorage } from '@vueuse/core'

const authStore = useAuthStore()
const snackbarStore = useSnackbarsStore()
const router = useRouter()

const emailInput: Ref<null | HTMLElement> = ref(null)
const form: Ref<null | VForm> = ref(null)

onMounted(() => {
  emailInput.value?.focus()
})

const rules = {
  mailRequired: (v: string) => !!v || 'Email is required.',
  passRequired: (v: string) => !!v || 'Password is required.',
  min: (v: string) => v.length >= 6 || `Min 6 characters`,
}

const isDev = useStorage('isDev', true)
const showPassword = ref(false)
const email = ref('')
const password = ref('')
const isSubmitted = ref(false)
const errorMessage: Ref<string | undefined> = ref(undefined)
const isLoading = ref(false)

async function login() {
  isLoading.value = true
  try {
    errorMessage.value = undefined
    // This will either succeed and navigate away, or throw an error.
    await authStore.login({
      email: email.value,
      password: password.value,
    })
    await router.push({ name: 'timeline' })
  } catch (e) {
    if (e instanceof Error) {
      snackbarStore.error('Could not log in', e as Error)
    } else if (isAxiosError(e)) {
      errorMessage.value = e.response?.data.error
    }
  } finally {
    // This will run whether the login succeeds or fails.
    isLoading.value = false
  }
}
</script>

<template>
  <focus-layout>
    <div class="login-container">
      <div class="left-pane">
        <div :class="{ rotating: isLoading }" class="big-image"></div>
      </div>
      <div class="right-pane">
        <div class="dev-buttons mb-5" v-if="isDev">
          <v-btn
            variant="tonal"
            rounded
            color="primary"
            @click="
              () => {
                email = 'ruurd@example.com'
                password = 'kibbeling'
                login()
              }
            "
            >Login Ruurd</v-btn
          >
          <v-btn
            variant="tonal"
            class="ml-3"
            rounded
            color="primary"
            @click="
              () => {
                email = 'other@example.com'
                password = 'kibbeling'
                login()
              }
            "
            >Login OtherUser</v-btn
          >
        </div>
        <v-form class="login-form" @submit.prevent="login()" ref="form">
          <v-text-field
            class="text-input"
            prepend-icon="mdi-email-outline"
            append-icon="empty"
            variant="outlined"
            ref="emailInput"
            rounded
            :rules="isSubmitted ? [rules.mailRequired, rules.min] : []"
            v-model="email"
            label="Email"
            color="primary"
            base-color="outline"
            placeholder="user@example.com"
            autocomplete="email"
          />
          <v-btn
            variant="plain"
            class="mb-1"
            color="primary"
            rounded
            tabindex="-1"
            density="compact"
            >Forgot password
          </v-btn>
          <v-text-field
            class="text-input mb-1"
            variant="outlined"
            :prepend-icon="showPassword ? 'mdi-lock-open-outline' : 'mdi-lock-outline'"
            :append-icon="showPassword ? 'mdi-eye-outline' : 'mdi-eye-off-outline'"
            @click:append="showPassword = !showPassword"
            :rules="isSubmitted ? [rules.passRequired] : []"
            :type="showPassword ? 'text' : 'password'"
            rounded
            v-model="password"
            color="primary"
            base-color="outline"
            label="Password"
            autocomplete="password"
          ></v-text-field>
          <v-btn
            class="mt-4"
            type="submit"
            variant="tonal"
            color="primary"
            rounded
            density="default"
            :loading="isLoading"
            width="150"
            >Login
          </v-btn>
        </v-form>

        <div class="mt-10 register-link text-caption">
          <span class="mr-2">Don't have an account?</span>
          <v-btn
            variant="plain"
            class="ml-2"
            color="primary"
            to="/register"
            rounded
            density="compact"
            >Register
          </v-btn>
        </div>

        <app-alert
          v-model="errorMessage"
          icon="mdi-alert-octagon"
          class="mt-8"
          background-color="error-container"
          text-color="on-error-container"
        />
      </div>
    </div>
  </focus-layout>
</template>

<style scoped>
.login-container {
  padding: 20px;
  display: flex;
}

.left-pane {
  width: 300px;
  display: flex;
  flex-direction: column;
}

.big-image {
  background-image: url('img/app-no-bg-1024.png');
  width: 100%;
  height: 100%;
  background-size: 80%;
  background-position: center;
}

.left-pane h1 {
  margin-left: 20px;
  font-size: 20px;
  font-weight: 500;
  opacity: 0.6;
}

.right-pane {
  width: 500px;
  margin-left: 50px;
}

.login-container > h1 {
  text-align: center;
}

.login-form {
  display: flex;
  align-items: center;
  flex-direction: column;
}

.text-input {
  width: 100%;
}

.rotating {
  animation: rotate 1s ease-in-out infinite;
}

@keyframes rotate {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.register-link {
  display: flex;
  justify-content: center;
  color: rgba(var(--v-theme-on-surface), 0.7);
  vertical-align: middle;
  line-height: 24px;
}
</style>
