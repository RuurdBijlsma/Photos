<script setup lang="ts">
import { type DialogAction, useDialogStore } from '@/scripts/stores/dialogStore'
const store = useDialogStore()

const handleAction = async (actionItem: DialogAction) => {
  if (actionItem.action) {
    await actionItem.action()
  }
  store.handleConfirm()
}
</script>

<template>
  <v-dialog
    :attach="store.current?.options.attach"
    :model-value="store.visible"
    @update:model-value="(val) => !val && store.handleCancel()"
    :persistent="store.current?.options.persistent"
    max-width="460"
  >
    <v-card v-if="store.current" rounded="xl" color="surface-container" class="dialog-card">
      <v-card-title class="d-flex align-center bg-surface-variant text-h6 py-3 px-4">
        <v-icon v-if="store.current.options.icon" :icon="store.current.options.icon" class="mr-3" />
        <span>{{ store.current.options.title }}</span>
        <v-spacer />
        <v-btn
          icon="mdi-close"
          variant="text"
          density="comfortable"
          @click="store.handleCancel()"
        />
      </v-card-title>

      <v-card-text class="pa-6 text-body-1">
        <p
          class="card-text"
          v-if="store.current.options.description"
          v-html="store.current.options.description"
        ></p>

        <v-text-field
          v-if="store.current.type === 'prompt'"
          v-model="store.inputValue"
          variant="outlined"
          base-color="primary"
          bg-color="surface-container-low"
          autofocus
          color="primary"
          density="comfortable"
          hide-details
          @keydown.enter="store.handleConfirm"
        />
      </v-card-text>

      <v-divider />

      <v-card-actions class="pa-3">
        <v-spacer />
        <v-btn
          v-if="store.current.type !== 'alert'"
          variant="text"
          rounded="lg"
          @click="store.handleCancel"
        >
          {{ store.current.options.cancelText || 'Cancel' }}
        </v-btn>

        <v-btn
          v-if="!store.current.options.actions"
          :color="store.current.options.color || 'primary'"
          variant="tonal"
          rounded="lg"
          @click="store.handleConfirm"
        >
          {{
            store.current.options.confirmText || (store.current.type === 'alert' ? 'OK' : 'Confirm')
          }}
        </v-btn>

        <v-btn
          v-else
          v-for="(action, i) in store.current.options.actions"
          :key="i"
          @click="handleAction(action)"
          :color="action.color || 'primary'"
          >{{ action.name }}</v-btn
        >
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
<style>
.card-text {
  margin: 0;
  margin-bottom: 5px;
  opacity: 0.9;
}
</style>
