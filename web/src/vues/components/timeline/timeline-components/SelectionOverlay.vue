<script setup lang="ts">
import MdiCheckboxMultipleMarkedCircleOutline from '~icons/mdi/checkbox-multiple-marked-circle-outline'
import MdiClose from '~icons/mdi/close'
import MdiDeleteForever from '~icons/mdi/delete-forever'
import MdiDeleteOutline from '~icons/mdi/delete-outline'
import MdiDotsHorizontal from '~icons/mdi/dots-horizontal'
import MdiRestore from '~icons/mdi/restore'
import AddToAlbumButton from '@/vues/components/timeline/timeline-components/AddToAlbumButton.vue'
import { useSelectionStore } from '@/scripts/stores/timeline/selectionStore.ts'
import type { TimelineContext } from '@/scripts/types/timeline/layout.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useProfileStore } from '@/scripts/stores/profileStore.ts'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useBinStore } from '@/scripts/stores/binStore.ts'
import { useMissingMediaStore } from '@/scripts/stores/missingMediaStore.ts'
import { useSystemStore } from '@/scripts/stores/systemStore.ts'
import { computed } from 'vue'
import { useDownloadStore } from '@/scripts/stores/downloadStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'
import { useResponsive } from '@/scripts/composables/useResponsive.ts'

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
const missingMediaStore = useMissingMediaStore()
const downloadStore = useDownloadStore()
const snackbarStore = useSnackbarsStore()
const mediaItemStore = useMediaItemStore()
const { isMobile } = useResponsive()

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

const searchSimilarUrl = computed(() => {
  return `/search?mode=similar&ids=${[...selectionStore.selection].join(',')}`
})

const SNACK_HEIGHT = 66
const SNACK_GAP = 8
const avoidSnackbarBottom = computed(() => {
  if (isMobile.value) {
    const count = Math.min(snackbarStore.snackQueue.length, 2)
    if (count === 0) return 0
    return count * (52 + 8) + 8
  }
  let increase = snackbarStore.snackQueue.length * (SNACK_HEIGHT + SNACK_GAP)
  if (snackbarStore.snackQueue.length > 0) increase += 16
  return increase
})
</script>

<template>
  <v-slide-y-reverse-transition>
    <div
      class="actions-overlay"
      v-if="selectionStore.selection.size > 0"
      :style="{
        transform: isMobile
          ? `translate(-50%, ${-1 * avoidSnackbarBottom}px)`
          : `translateY(${-1 * avoidSnackbarBottom}px)`,
      }"
    >
      <v-btn
        :icon="MdiClose"
        variant="plain"
        density="compact"
        v-tooltip:top="'Deselect'"
        @click="selectionStore.deselectAll"
      />
      <v-btn
        :icon="MdiCheckboxMultipleMarkedCircleOutline"
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
      <template v-if="!context?.isBin && !context?.isMissing">
        <add-to-album-button
          :exclude-album-ids="excludeAlbumIds"
          :ids-to-add="[...selectionStore.selection]"
        />
        <v-btn
          :icon="MdiDeleteOutline"
          variant="plain"
          density="compact"
          v-tooltip:top="'Move to bin'"
          :loading="binStore.softDeleteLoading"
          @click="binStore.softDeleteItems([...selectionStore.selection])"
        />

        <v-menu>
          <template v-slot:activator="{ props }">
            <v-btn v-bind="props" :icon="MdiDotsHorizontal" variant="plain" density="compact" />
          </template>
          <v-list density="compact">
            <v-list-item v-if="selectionStore.selection.size === 1" @click="setProfilePic">
              <v-list-item-title>Set as profile picture</v-list-item-title>
            </v-list-item>
            <v-list-item
              v-if="selectionStore.selection.size === 1"
              @click="mediaItemStore.reprocessMediaItem([...selectionStore.selection][0])"
            >
              <v-list-item-title>Reprocess media item</v-list-item-title>
            </v-list-item>
            <v-list-item :to="searchSimilarUrl">
              <v-list-item-title>Find similar images</v-list-item-title>
            </v-list-item>
            <v-list-item @click="downloadStore.multiDownloadItems([...selectionStore.selection])">
              <v-list-item-title>Download</v-list-item-title>
            </v-list-item>
            <!-- Album specific list items -->
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

      <!-- Missing-specific Actions -->
      <template v-else-if="context?.isMissing">
        <v-btn
          :icon="MdiDeleteForever"
          variant="plain"
          density="compact"
          v-tooltip:top="'Prune from database'"
          :loading="missingMediaStore.pruning"
          @click="missingMediaStore.pruneItems([...selectionStore.selection])"
        />
      </template>

      <!-- Bin-specific Actions -->
      <template v-else>
        <v-btn
          :icon="MdiRestore"
          variant="plain"
          density="compact"
          v-tooltip:top="'Restore'"
          :loading="binStore.restoreLoading"
          @click="binStore.restoreItems([...selectionStore.selection])"
        />
        <v-btn
          v-if="systemStore.stats.allowFileDeletion"
          :icon="MdiDeleteForever"
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
  --width: 468px;
  position: fixed;
  bottom: 20px;
  right: 26px;
  width: var(--width);
  height: 70px;
  padding: 10px 20px;
  z-index: 1005;
  text-align: left;
  font-weight: 500;
  border-radius: 40px;
  background-color: rgba(var(--v-theme-surface-container-high), 1);
  color: rgba(var(--v-theme-on-surface-container-high), 1);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  transition: transform 0.3s cubic-bezier(0.25, 0.8, 0.25, 1);
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
}

.bold-select {
  font-weight: 600;
}

.select-text {
  white-space: nowrap;
}

.is-mobile .actions-overlay {
  bottom: calc(84px + env(safe-area-inset-bottom, 0px));
  left: 50%;
  right: auto;
  width: calc(100vw - 32px);
  max-width: 440px;
  height: 56px;
  padding: 6px 12px;
  border-radius: 28px;
  gap: 8px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.35);
}

.is-mobile .select-text {
  font-size: 0.85rem;
}
</style>
