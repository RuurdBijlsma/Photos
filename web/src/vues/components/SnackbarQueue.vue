<script setup lang="ts">
import { ref } from 'vue'
import { isAxiosError } from 'axios'
import { type Snack, useSnackbarsStore } from '@/scripts/stores/snackbarStore'

const store = useSnackbarsStore()

// For the Error Details Modal
const dialog = ref(false)
const selectedSnack = ref<Snack | null>(null)

const showErrorDetails = (snack: Snack) => {
  store.pauseTimeout(snack.id)
  selectedSnack.value = snack
  dialog.value = true
}

// Handle Custom Actions
const handleAction = async (snack: Snack) => {
  store.pauseTimeout(snack.id)
  if (snack.action?.onClick) {
    await snack.action.onClick()
  }
  if (snack.action?.hideOnClick === undefined || snack.action?.hideOnClick === true) {
    store.remove(snack.id)
  }
}

const onMouseEnter = (id: string) => store.pauseTimeout(id)
const onMouseLeave = (id: string) => store.resumeTimeout(id)
</script>

<template>
  <TransitionGroup name="snack" tag="div" class="snackbar-queue-container">
    <div
      v-for="snack in store.snackQueue"
      :key="snack.id"
      class="snack-wrapper"
      @mouseenter="onMouseEnter(snack.id)"
      @mouseleave="onMouseLeave(snack.id)"
    >
      <v-alert
        :color="snack.color"
        :icon="snack.loading ? false : snack.icon || false"
        variant="flat"
        rounded="pill"
        class="snack-alert"
      >
        <!-- Loading Spinner Prepend -->
        <template v-if="snack.loading" #prepend>
          <v-progress-circular indeterminate size="20" width="2" class="mr-2" />
        </template>

        <div class="snack-content-wrapper">
          <div class="snack-message">
            {{ snack.message }}
          </div>

          <div class="snack-actions">
            <!-- Detailed Error Button -->
            <v-btn
              v-if="snack.error || snack.errorData"
              icon="mdi-information-outline"
              variant="text"
              density="comfortable"
              size="small"
              class="action-btn"
              @click.stop="showErrorDetails(snack)"
            />
            <v-btn
              v-if="snack.action"
              :text="snack.action.label"
              @click.stop="handleAction(snack)"
              density="comfortable"
              rounded
              variant="tonal"
            />
            <!-- Dismiss Button (hidden if dismissable is false) -->
            <v-btn
              v-if="snack.dismissable !== false"
              icon="mdi-close"
              variant="text"
              density="comfortable"
              size="small"
              class="action-btn close-btn"
              @click.stop="store.remove(snack.id)"
            />
          </div>
        </div>
      </v-alert>
    </div>
  </TransitionGroup>

  <!-- Detailed Error Dialog -->
  <v-dialog v-model="dialog" max-width="700">
    <v-card v-if="selectedSnack && selectedSnack.error">
      <v-toolbar color="error" density="compact">
        <v-toolbar-title class="text-subtitle-1">
          {{ selectedSnack.error.message || 'Error Details' }}
        </v-toolbar-title>
        <v-spacer />
        <v-btn icon="mdi-close" @click="dialog = false" />
      </v-toolbar>

      <v-card-text class="pt-4">
        <v-alert v-if="selectedSnack.errorData?.error" type="warning" variant="tonal" class="mb-4">
          <strong>Server Message:</strong> {{ selectedSnack.errorData.error }}
        </v-alert>

        <p class="mb-2"><strong>Message:</strong> {{ selectedSnack.error.message }}</p>
        <p
          v-if="isAxiosError(selectedSnack.error) && selectedSnack.error.response?.data"
          class="mb-2"
        >
          <strong>Axios:</strong> {{ selectedSnack.error.response?.data }}
        </p>

        <v-expansion-panels v-if="selectedSnack.error.stack">
          <v-expansion-panel>
            <v-expansion-panel-title>Stack Trace</v-expansion-panel-title>
            <v-expansion-panel-text>
              <div class="stack-trace bg-grey-lighten-4 pa-2 text-caption">
                {{ selectedSnack.error.stack }}
              </div>
            </v-expansion-panel-text>
          </v-expansion-panel>
        </v-expansion-panels>
      </v-card-text>

      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" @click="dialog = false">Close</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.snackbar-queue-container {
  position: fixed;
  bottom: 24px;
  right: 60px;
  z-index: 6000;
  pointer-events: none;
  width: 100%;
  max-width: 500px;
  padding: 0 16px;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
}

.snack-wrapper {
  pointer-events: auto;
  width: 100%;
  transition: all 0.4s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.snack-alert {
  margin: 0 !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.snack-alert :deep(.v-alert__content) {
  width: 100%;
}

.snack-content-wrapper {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  gap: 16px;
}

.snack-message {
  font-size: 0.875rem;
  font-weight: 500;
  line-height: 1.4;
  word-break: break-word;
}

.snack-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.snack-enter-from {
  opacity: 0;
  transform: translateY(30px) scale(0.9);
}

.snack-leave-to {
  opacity: 0;
  transform: translateY(-20px) scale(0.9);
}

.snack-leave-active {
  position: absolute !important;
  left: 16px;
  right: 16px;
  margin: 0 auto;
}

.snack-move {
  transition: transform 0.4s cubic-bezier(0.25, 0.8, 0.25, 1);
}

.stack-trace {
  font-family: monospace;
  white-space: pre-wrap;
  overflow-x: auto;
  max-height: 200px;
}
</style>
