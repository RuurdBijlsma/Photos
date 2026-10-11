<script setup lang="ts">
import MdiCloudUpload from '~icons/mdi/cloud-upload'
import MdiEarth from '~icons/mdi/earth'
import MdiLock from '~icons/mdi/lock'
import MdiMinus from '~icons/mdi/minus'
import MdiShare from '~icons/mdi/share'
import { computed, ref, watch } from 'vue'
import type { AlbumInfo, CollaboratorSummary } from '@/scripts/types/generated/timeline.ts'
import type { SmallUser } from '@/scripts/types/api/user.ts'
import UserAvatar from '@/vues/components/ui/UserAvatar.vue'
import userService from '@/scripts/services/userService.ts'
import albumService from '@/scripts/services/albumService.ts'
import { useAlbumStore } from '@/scripts/stores/albumStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { copyToClipboard } from '@/scripts/utils.ts'

const props = defineProps<{
  album: AlbumInfo
  isOwner: boolean
  currentUserCollaborator: CollaboratorSummary | undefined
}>()

const emit = defineEmits<{
  (e: 'publicize'): void
  (e: 'privatize'): void
}>()

const albumStore = useAlbumStore()
const snackbars = useSnackbarsStore()
const dialogs = useDialogStore()

const collabMenuOpen = ref(false)
const allUsers = ref<SmallUser[]>([])

const otherUsers = computed(() =>
  allUsers.value.filter((u) => !props.album.collaborators.map((c) => c.userId).includes(u.id)),
)

async function fetchUserList() {
  try {
    const { data } = await userService.listUsers()
    allUsers.value = data
  } catch (e) {
    snackbars.error("Can't fetch other users", e)
  }
}

async function addCollaborator(userId: number) {
  try {
    await albumService.addCollaborator(props.album.id, {
      userId,
      role: 'Contributor',
    })
    requestIdleCallback(() => {
      albumStore.fetchAlbumMedia(props.album.id, false)
    })
  } catch (e) {
    snackbars.error('Could not add collaborator', e)
  }
}

async function crossServerInvite() {
  try {
    const { data: token } = await albumService.generateInvite(props.album.id)
    await dialogs.alert({
      title: 'Cross-Server Share',
      description: `
        <div>
          <p>Send this token to someone. They can paste it into the search bar on their own Ruurd Photos instance to import this album.</p>
          <br>
          <code class="share-link">${token}</code>
        </div>
      `,
      actions: [
        {
          action: () => copyToClipboard(token),
          name: 'Copy Token',
        },
        { action: () => ({}), name: 'Done' },
      ],
    })
  } catch (e) {
    snackbars.error("Can't generate invite", e)
  }
}

watch(collabMenuOpen, () => {
  if (collabMenuOpen.value) {
    fetchUserList()
  }
})
</script>

<template>
  <div class="album-collaborators">
    <v-menu v-for="collaborator in album.collaborators" :key="collaborator.id">
      <template v-slot:activator="{ props: menuProps }">
        <user-avatar
          v-bind="collaborator.userId !== album.ownerId && isOwner ? menuProps : undefined"
          :name="collaborator.name"
          :avatar-id="collaborator.avatarId"
          class="collaborator-avatar"
          v-tooltip:top="
            `${collaborator.name}${collaborator.userId === album.ownerId ? ' (Owner)' : ''}`
          "
        />
      </template>
      <v-list density="compact">
        <v-list-item @click="albumStore.removeCollaborator(album.id, collaborator.id, false)">
          <v-list-item-title>Remove {{ collaborator.name }}</v-list-item-title>
        </v-list-item>
      </v-list>
    </v-menu>

    <!-- Owner Menu -->
    <v-menu v-model="collabMenuOpen" v-if="isOwner">
      <template v-slot:activator="{ props: menuProps }">
        <v-btn
          v-bind="menuProps"
          v-tooltip:top="'Add collaborator'"
          :icon="MdiShare"
          variant="tonal"
          color="primary"
          size="40"
          :loading="allUsers.length === 0 && collabMenuOpen"
        />
      </template>
      <v-list density="compact">
        <template v-if="otherUsers.length > 0">
          <v-list-subheader class="mb-1">Add a collaborator</v-list-subheader>
          <v-list-item
            @click="addCollaborator(user.id)"
            v-for="user in otherUsers"
            :key="user.id"
            class="collab-list-item"
          >
            <user-avatar :name="user.name" :avatar-id="user.avatarId" />
            <v-list-item-title>{{ user.name }}</v-list-item-title>
          </v-list-item>
          <v-divider class="mt-2 mb-1" />
        </template>
        <v-list-item
          :prepend-icon="MdiLock"
          @click="emit('privatize')"
          v-if="album.isPublic"
          v-tooltip="{
            location: 'top',
            text: 'Only invited people will be able to access this album.',
          }"
        >
          <v-list-item-title>Make private</v-list-item-title>
        </v-list-item>
        <v-list-item
          :prepend-icon="MdiEarth"
          @click="emit('publicize')"
          v-else
          v-tooltip="{
            location: 'top',
            text: 'Anyone with the link will be able to view this album.',
          }"
        >
          <v-list-item-title>Make public</v-list-item-title>
        </v-list-item>
        <v-list-item
          :prepend-icon="MdiCloudUpload"
          @click="crossServerInvite"
          v-tooltip="{
            location: 'top',
            text: 'Export this album to another server.',
          }"
        >
          <v-list-item-title>Share with other server</v-list-item-title>
        </v-list-item>
      </v-list>
    </v-menu>

    <!-- Contributor Leave Button -->
    <v-menu v-else-if="currentUserCollaborator">
      <template v-slot:activator="{ props: menuProps }">
        <v-btn
          v-bind="menuProps"
          v-tooltip:top="'Leave album'"
          :icon="MdiMinus"
          variant="tonal"
          color="primary"
          size="40"
        />
      </template>
      <v-list density="compact">
        <v-list-item
          @click="albumStore.removeCollaborator(album.id, currentUserCollaborator.id, true)"
          class="collab-list-item"
        >
          <v-list-item-title>Leave album</v-list-item-title>
        </v-list-item>
      </v-list>
    </v-menu>
  </div>
</template>

<style scoped>
.album-collaborators {
  margin-top: 5px;
  display: flex;
  gap: 10px;
}

.collaborator-avatar {
  font-weight: 600;
}

.collab-list-item:deep(.v-list-item__content) {
  display: flex;
  align-items: center;
  gap: 20px;
}
</style>
