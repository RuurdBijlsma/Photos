<script setup lang="ts">
import { usePickFolderStore } from '@/scripts/stores/pickFolderStore.ts'
import { useAdminStore } from '@/scripts/stores/adminStore.ts'

const adminStore = useAdminStore()
const pickFolderStore = usePickFolderStore()
</script>

<template>
  <v-card class="folder-card" variant="text" rounded color="primary">
    <v-card-title class="d-flex align-center card-title"> Process media library </v-card-title>
    <v-card-text>
      <v-table striped="odd">
        <tbody>
          <tr>
            <td class="table-left">Media folder</td>
            <td class="table-right">{{ adminStore.disks?.mediaFolder?.folder }}</td>
          </tr>
          <tr>
            <td class="table-left">Thumbnails folder</td>
            <td class="table-right">{{ adminStore.disks?.appDataFolder?.folder }}</td>
          </tr>
          <tr>
            <td class="table-left">User folder</td>
            <td class="table-right">{{ '/' + pickFolderStore.viewedFolder.join('/') }}</td>
          </tr>
        </tbody>
      </v-table>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.table-left {
  font-weight: 500;
}
.table-right {
  font-weight: 400;
}
</style>
