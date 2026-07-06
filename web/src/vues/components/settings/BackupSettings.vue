<script setup lang="ts">
import { onMounted } from 'vue'
import { useAlbumBackupStore } from '@/scripts/stores/albumBackupStore.ts'

const backupStore = useAlbumBackupStore()

onMounted(() => {
  backupStore.fetchBackups()
})

function formatBytes(bytes: number) {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}
</script>

<template>
  <div class="backup-settings-layout">
    <!-- Settings Configuration Panel -->
    <section class="config-panel">
      <v-card class="settings-card" flat border>
        <!-- Card Header -->
        <div class="card-header">
          <span class="card-title">System Backup & Recovery</span>
          <v-icon color="primary" size="large">mdi-backup-restore</v-icon>
        </div>

        <div class="card-body">
          <!-- Section: Album Backups -->
          <div class="section-divider">
            <span class="section-label">Album Backups</span>
            <v-divider class="divider-line" />
          </div>

          <p class="section-desc mb-4">
            Recreate missing albums from stored backups. Album backups are created automatically
            when you change a user's media folder. Restoring will merge metadata without deleting
            existing photos.
          </p>

          <v-list v-if="backupStore.backups.length > 0" class="backup-list" bg-color="transparent">
            <v-list-item
              v-for="backup in backupStore.backups"
              :key="backup.filename"
              class="backup-item mb-2"
              rounded="lg"
              border
            >
              <template v-slot:prepend>
                <v-icon color="secondary" class="mr-2">mdi-file-document-outline</v-icon>
              </template>

              <v-list-item-title class="backup-filename">
                {{ backup.filename }}
              </v-list-item-title>
              <v-list-item-subtitle class="backup-meta">
                {{ formatDate(backup.createdAt) }} &bull; {{ formatBytes(backup.sizeBytes) }}
              </v-list-item-subtitle>

              <template v-slot:append>
                <v-btn
                  color="primary"
                  variant="tonal"
                  rounded
                  size="small"
                  prepend-icon="mdi-restore"
                  @click="backupStore.restoreBackup(backup.filename)"
                >
                  Restore
                </v-btn>
              </template>
            </v-list-item>
          </v-list>

          <v-alert v-else variant="tonal" type="info" rounded="xl" class="no-backups-alert">
            No album backups were found on the server.
          </v-alert>

          <!-- Expansion hook: Additional backup/restore operations can be placed here -->
        </div>
      </v-card>
    </section>
  </div>
</template>

<style scoped>
.backup-settings-layout {
  display: grid;
  grid-template-columns: 1fr;
  gap: 24px;
}

.settings-card {
  background-color: rgb(var(--v-theme-surface-container-low)) !important;
  border-radius: 24px !important;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.card-header {
  background-color: rgb(var(--v-theme-surface-container-high));
  padding: 16px 24px;
  border-top-left-radius: 24px;
  border-top-right-radius: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.card-title {
  font-size: 1.25rem;
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface));
}

.card-body {
  padding: 24px;
}

.section-divider {
  display: flex;
  align-items: center;
  margin-bottom: 16px;
}

.section-label {
  font-size: 0.9rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  color: rgb(var(--v-theme-primary));
  white-space: nowrap;
}

.divider-line {
  margin-left: 16px;
  opacity: 0.3;
}

.section-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
  line-height: 1.4;
}

.backup-list {
  padding: 0;
}

.backup-item {
  background-color: rgb(var(--v-theme-surface-container-high)) !important;
  border-color: rgba(var(--v-border-color), var(--v-border-opacity)) !important;
}

.backup-filename {
  font-weight: 600;
  font-size: 0.95rem !important;
}

.backup-meta {
  font-size: 0.8rem !important;
  opacity: 0.8;
}

.no-backups-alert {
  border-radius: 16px;
}

.info-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
  margin-bottom: 16px;
}

.info-item {
  display: flex;
  align-items: flex-start;
}

.info-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
  margin-bottom: 4px;
}

.info-text {
  font-size: 0.8rem;
  color: rgb(var(--v-theme-on-surface-variant));
  line-height: 1.4;
}
</style>
