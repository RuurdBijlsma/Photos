<script setup lang="ts">
import MdiArrowUp from '~icons/mdi/arrow-up'
import MdiChevronRight from '~icons/mdi/chevron-right'
import MdiFolderOutline from '~icons/mdi/folder-outline'
import MdiFolderPlusOutline from '~icons/mdi/folder-plus-outline'
import MdiRefresh from '~icons/mdi/refresh'
import { onMounted, watch } from 'vue'
import { usePickFolderStore } from '@/scripts/stores/pickFolderStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'

const pickFolderStore = usePickFolderStore()
const dialogs = useDialogStore()

watch(
  () => pickFolderStore.viewedFolder,
  () => onViewedChange(),
  { deep: true },
)

async function onViewedChange() {
  setTimeout(() => {
    const el = document.querySelector('.current-route-display')
    if (el) el.scrollLeft = el.scrollWidth - el.clientWidth
  }, 50)
}

async function promptCreateFolder() {
  const folder = await dialogs.prompt({
    title: 'Create folder',
    description: 'Create folder',
    icon: MdiFolderPlusOutline,
    confirmText: 'Create',
  })
  if (!folder) return
  await pickFolderStore.makeFolder(folder)
}

async function init() {
  await pickFolderStore.refreshFolders()
}

onMounted(init)
</script>

<template>
  <v-card variant="flat" class="folder-picker" color="surface-container">
    <v-card-text>
      <div class="picker-header">
        <div class="header-buttons">
          <v-btn
            color="primary"
            class="mr-2"
            variant="text"
            :disabled="
              pickFolderStore.viewedFolder.length === 0 || pickFolderStore.listFolderLoading
            "
            @click="pickFolderStore.truncateViewed(pickFolderStore.viewedFolder.length - 1)"
            density="compact"
            :icon="MdiArrowUp"
          />
          <v-btn
            color="primary"
            class="mr-2"
            variant="text"
            title="Create folder"
            density="compact"
            :icon="MdiFolderPlusOutline"
            :loading="pickFolderStore.makeFolderLoading"
            :disabled="pickFolderStore.listFolderLoading"
            @click="promptCreateFolder"
          />
        </div>
        <div
          class="current-route-display"
          :class="{ 'is-loading': pickFolderStore.listFolderLoading }"
        >
          <div
            class="route-component route-root"
            v-ripple="!pickFolderStore.listFolderLoading"
            @click="!pickFolderStore.listFolderLoading && pickFolderStore.truncateViewed(0)"
          >
            Media Root
          </div>
          <template v-for="(component, index) in pickFolderStore.viewedFolder" :key="index">
            <v-icon :icon="MdiChevronRight" />
            <div
              class="route-component"
              v-ripple="!pickFolderStore.listFolderLoading"
              @click="
                !pickFolderStore.listFolderLoading && pickFolderStore.truncateViewed(index + 1)
              "
            >
              {{ component }}
            </div>
          </template>
        </div>
        <div class="header-buttons">
          <v-btn
            color="primary"
            class="ml-2"
            variant="text"
            density="compact"
            :icon="MdiRefresh"
            @click="pickFolderStore.refreshFolders"
            :loading="pickFolderStore.listFolderLoading"
          />
        </div>
      </div>

      <div class="picker-entries mt-5">
        <!-- Loading State -->
        <div
          v-if="pickFolderStore.listFolderLoading"
          class="d-flex flex-column align-center justify-center fill-height loading-container"
        >
          <v-progress-circular indeterminate color="primary" size="32" />
          <span class="text-caption text-medium-emphasis mt-2">Loading folders...</span>
        </div>

        <!-- Empty State -->
        <p
          class="text-caption text-center font-italic mt-8"
          v-else-if="pickFolderStore.folderList.length === 0"
        >
          There are no folders here.
        </p>

        <!-- Folder List -->
        <template v-else>
          <v-list-item
            v-for="folder in pickFolderStore.folderList"
            :key="folder"
            class="rounded-xl"
            @click="pickFolderStore.openFolder(folder)"
            :prepend-icon="MdiFolderOutline"
            :title="folder"
          />
        </template>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.folder-picker {
  border-radius: 50px;
  padding: 15px;
  margin-left: -15px;
  margin-right: -15px;
}

.picker-header {
  display: flex;
  align-items: center;
}

.current-route-display {
  display: flex;
  border-radius: 15px;
  padding: 2px 7px;
  flex-grow: 1;
  font-size: 14px;
  font-weight: 500;
  align-items: center;
  background-color: rgba(0, 0, 0, 0.08);
  overflow-x: auto;
  scroll-behavior: smooth;
  transition: opacity 0.2s ease;
}

.current-route-display.is-loading {
  opacity: 0.7;
}

.current-route-display::-webkit-scrollbar {
  display: none;
}

.route-root {
  font-weight: bold;
  opacity: 0.5;
}

.route-component {
  cursor: pointer;
  white-space: nowrap;
  padding: 2px 8px;
  border-radius: 15px;
}

.route-component:hover {
  text-decoration: underline;
}

.header-buttons {
  opacity: 0.6;
}

.header-buttons:first-child {
  min-width: 75px;
}

.picker-entries {
  height: 180px;
  max-height: 340px;
  overflow-y: auto;
}

.loading-container {
  min-height: 140px;
}
</style>
