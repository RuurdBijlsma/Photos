<script setup lang="ts">
import AddToAlbumButton from '@/vues/components/timeline/timeline-components/AddToAlbumButton.vue'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import type { TimelineContext } from '@/scripts/types/timeline/layout.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useProfileStore } from '@/scripts/stores/profileStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useBinStore } from '@/scripts/stores/binStore.ts'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'

withDefaults(
  defineProps<{
    excludeAlbumIds?: string[]
    context?: TimelineContext
  }>(),
  {
    excludeAlbumIds: () => [],
    context: () => ({}),
  },
)

const profileStore = useProfileStore()
const systemStore = useSystemStore()
const selectionStore = useSelectionStore()
const albumStore = useAlbumStore()
const authStore = useAuthStore()
const binStore = useBinStore()

async function setProfilePic() {
  if (selectionStore.selection.size !== 1) return
  const mediaItemId = [...selectionStore.selection][0]
  await profileStore.setProfilePic(mediaItemId)
}

async function setAlbumCover(albumId: string) {
  if (selectionStore.selection.size !== 1) return
  const mediaItemId = [...selectionStore.selection][0]
  await albumStore.updateAlbumDetails(albumId, { thumbnailId: mediaItemId })
  requestIdleCallback(() => {
    albumStore.fetchAlbumMedia(albumId, false)
    albumStore.fetchUserAlbums()
  })
}
</script>

<template>
  <v-slide-y-reverse-transition>
    <div class="actions-overlay" v-if="selectionStore.selection.size > 0">
      <v-btn
        icon="mdi-close"
        variant="plain"
        density="compact"
        v-tooltip:top="'Deselect all'"
        @click="selectionStore.deselectAll"
      />
      <v-btn
        icon="mdi-checkbox-multiple-marked-circle-outline"
        variant="plain"
        density="compact"
        @click="selectionStore.selectAll"
        v-tooltip:top="'Select all'"
      />
      <div class="select-text">
        <span class="bold-select">{{ selectionStore.selection.size }}</span
        ><span> selected</span>
      </div>
      <v-spacer />

      <!-- Regular Actions -->
      <template v-if="!context?.isBin">
        <add-to-album-button
          :exclude-album-ids="excludeAlbumIds"
          :ids-to-add="[...selectionStore.selection]"
        />
        <v-btn
          icon="mdi-delete-outline"
          variant="plain"
          density="compact"
          v-tooltip:top="'Move to bin'"
          :loading="binStore.softDeleteLoading"
          @click="binStore.softDeleteItems([...selectionStore.selection])"
        />

        <v-menu>
          <template v-slot:activator="{ props }">
            <v-btn v-bind="props" icon="mdi-dots-horizontal" variant="plain" density="compact" />
          </template>
          <v-list density="compact">
            <v-list-item v-if="selectionStore.selection.size === 1" @click="setProfilePic">
              <v-list-item-title>Set as profile picture</v-list-item-title>
            </v-list-item>
            <template v-if="context && context.album">
              <v-divider />
              <v-list-subheader>Album</v-list-subheader>
              <v-list-item
                @click="albumStore.removeFromAlbum(context.album.id, [...selectionStore.selection])"
              >
                <v-list-item-title>Remove from album</v-list-item-title>
              </v-list-item>
              <v-list-item
                v-if="
                  selectionStore.selection.size === 1 &&
                  context.album.ownerId === authStore.user?.id
                "
                @click="setAlbumCover(context.album.id)"
              >
                <v-list-item-title>Set as album cover</v-list-item-title>
              </v-list-item>
            </template>
          </v-list>
        </v-menu>
      </template>

      <!-- Bin-specific Actions -->
      <template v-else>
        <v-btn
          icon="mdi-restore"
          variant="plain"
          density="compact"
          v-tooltip:top="'Restore'"
          :loading="binStore.restoreLoading"
          @click="binStore.restoreItems([...selectionStore.selection])"
        />
        <v-btn
          v-if="systemStore.stats.allowFileDeletion"
          icon="mdi-delete-forever"
          variant="plain"
          density="compact"
          v-tooltip:top="'Delete permanently'"
          :loading="binStore.hardDeleteLoading"
          @click="binStore.hardDeleteItems([...selectionStore.selection])"
        />
      </template>
    </div>
  </v-slide-y-reverse-transition>
</template>

<style scoped>
.actions-overlay {
  --width: 400px;
  position: absolute;
  bottom: 30px;
  margin-left: calc(50% - var(--width) / 2);
  width: var(--width);
  height: 70px;
  padding: 10px 20px;
  z-index: 3;
  text-align: left;
  font-weight: 500;
  border-radius: 40px;
  background-color: rgba(var(--v-theme-surface-container-high), 1);
  color: rgba(var(--v-theme-on-surface-container-high), 1);
  box-shadow:
    0 4px 8px 0 rgba(0, 0, 0, 0.2),
    0 6px 20px 0 rgba(0, 0, 0, 0.19);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.bold-select {
  font-weight: 600;
}
</style>
