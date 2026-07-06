<script setup lang="ts">
import type { PathInfoResponse } from '@/scripts/types/api/admin.js'
import { prettyBytes } from '@/scripts/utils.ts'

defineProps<{
  folder: PathInfoResponse
  envVar: string
  titleIcon: string
}>()
</script>

<template>
  <v-card class="mb-6 folder-card pa-3" variant="flat" color="surface-container">
    <v-card-title class="d-flex align-center card-title">
      <v-icon size="22" :icon="titleIcon" class="mr-5"></v-icon>
      {{ folder.folder }}
      <v-spacer />
      <v-chip
        class="ml-2"
        :color="folder.readAccess ? 'secondary-container' : 'error'"
        size="small"
      >
        {{ folder.readAccess ? 'Connected' : 'No Access' }}
      </v-chip>
    </v-card-title>
    <v-card-text>
      <div>
        <div class="d-flex justify-space-between mb-2 disk-text-container mt-3">
          <span class="text-caption disk-uppercase">{{ envVar }} </span>
          <span class="text-caption black-text"
            >{{ prettyBytes(folder.diskUsed) }} / {{ prettyBytes(folder.diskTotal) }}</span
          >
        </div>
        <v-progress-linear
          :model-value="(folder.diskUsed / folder.diskTotal) * 100"
          class="linear-progress mt-0"
          color="primary"
          rounded
        />

        <div class="chips">
          <v-chip
            variant="tonal"
            density="compact"
            :color="folder.readAccess ? 'primary' : 'error'"
          >
            <v-icon class="mr-2" :icon="folder.readAccess ? 'mdi-check' : 'mdi-close'"></v-icon>
            Read
          </v-chip>
          <v-chip
            variant="tonal"
            density="compact"
            :color="folder.writeAccess ? 'primary' : 'error'"
          >
            <v-icon class="mr-2" :icon="folder.writeAccess ? 'mdi-check' : 'mdi-close'"></v-icon>
            Write
          </v-chip>
        </div>
      </div>
      <div v-if="!folder.readAccess || !folder.writeAccess" class="mt-4 text-error">
        <v-icon icon="mdi-alert" class="mr-3"></v-icon>
        This folder has permission issues. Please check read/write access.
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.card-title {
  font-weight: 600;
  font-size: 17px;
  font-family: Arial, Helvetica, Roboto, sans-serif;
  color: rgb(var(--v-theme-on-surface-variant));
}

.text-error {
  color: #ff5252;
  font-weight: 500;
}

.folder-card {
  border-radius: 24px;
}

.chips {
  margin-top: 15px;
  display: flex;
  gap: 15px;
}

.linear-progress {
  opacity: 0.7;
}

.disk-uppercase {
  text-transform: uppercase;
  font-weight: bold;
  opacity: 0.7;
  color: rgb(var(--v-theme-on-surface-variant));
}

.black-text {
  color: rgb(var(--v-theme-on-surface-variant));
}
</style>
