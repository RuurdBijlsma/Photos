<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import UserAvatar from '@/vues/components/ui/UserAvatar.vue'
import { useAuthStore } from '@/scripts/stores/authStore.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import userService from '@/scripts/services/userService.ts'
import type { UserProfile } from '@/scripts/types/api/user.ts'
import { useDate } from 'vuetify'
import { usePeopleStore } from '@/scripts/stores/peopleStore.ts'
import peopleService from '@/scripts/services/peopleService.ts'

const authStore = useAuthStore()
const snackbars = useSnackbarsStore()
const peopleStore = usePeopleStore()
const router = useRouter()
const route = useRoute()
const date = useDate()

const loading = ref(true)
const profile = ref<UserProfile | null>(null)
const showExplanation = ref<boolean>(false)

const isCurrentUser = computed(() => {
  return authStore.user?.id === Number(route.params.userId)
})

async function loadProfile() {
  loading.value = true
  try {
    const userId = Number(route.params.userId)
    if (isNaN(userId)) throw new Error('Invalid User ID')
    const response = await userService.getUserProfile(userId)
    profile.value = response.data
  } catch (error) {
    snackbars.error('Failed to load profile', error)
    console.error(error)
  } finally {
    loading.value = false
  }
}

const editDialog = ref(false)
const editName = ref('')
const editAvatar = ref<string | null>(null)
const saving = ref(false)

function openEditDialog() {
  if (!profile.value) return
  editName.value = profile.value.name
  editAvatar.value = profile.value.avatarId
  editDialog.value = true
}

async function saveProfile() {
  if (!editName.value.trim() || !profile.value) return

  saving.value = true
  try {
    const response = await userService.updateProfile({
      name: editName.value.trim(),
      avatarId: editAvatar.value?.trim(),
    })
    profile.value = response.data
    // Update auth store too if it's the current user
    if (isCurrentUser.value) {
      await authStore.fetchCurrentUser()
    }
    editDialog.value = false
  } catch (error) {
    snackbars.error('Failed to update profile')
    console.error(error)
  } finally {
    saving.value = false
  }
}

function formatDate(dateStr: string) {
  if (!dateStr) return ''
  return date.format(new Date(dateStr), 'monthAndYear')
}

let peopleIdsTried: string[] = []

async function autoSetProfilePic() {
  await peopleStore.fetchPeople()
  if (peopleStore.people.length === 0) return
  const ids = peopleStore.people.map((p) => p.id).filter((pid) => !peopleIdsTried.includes(pid))
  const personId = ids[0]
  peopleIdsTried.push(personId)
  if (ids.length === 1) {
    peopleIdsTried = []
  }
  try {
    const { data } = await peopleService.getMediaItemId(personId)
    editAvatar.value = data
  } catch {
    snackbars.warning("Couldn't get profile picture")
  }
}

watch(
  () => route.params.userId,
  () => {
    loadProfile()
  },
  { immediate: true },
)

const statCards = computed(() => [
  {
    title: 'Photos',
    value: profile.value?.stats.photoCount ?? 0,
    icon: 'mdi-image-multiple',
    color: 'primary',
  },
  {
    title: 'Videos',
    value: profile.value?.stats.videoCount ?? 0,
    icon: 'mdi-video',
    color: 'secondary',
  },
  {
    title: 'Albums',
    value: profile.value?.stats.albumCount ?? 0,
    icon: 'mdi-image-album',
    color: 'tertiary',
  },
  {
    title: 'Shared',
    value: profile.value?.stats.sharedAlbumCount ?? 0,
    icon: 'mdi-share-variant',
    color: 'info',
  },
])
</script>

<template>
  <main-layout-container class="profile-view" fit-to-content>
    <div v-if="loading" class="loading-state">
      <v-progress-circular indeterminate color="primary" size="64" />
    </div>

    <div v-else-if="profile" class="profile-content">
      <!-- Header Section -->
      <section class="profile-header">
        <user-avatar :name="profile.name" :avatar-id="profile.avatarId" :size="150" elevation="4" />
        <div class="user-info">
          <div class="user-title-row">
            <h1 class="user-name">{{ profile.name }}</h1>
            <v-btn
              v-if="isCurrentUser"
              prepend-icon="mdi-pencil"
              variant="tonal"
              rounded="xl"
              class="edit-btn"
              color="primary"
              @click="openEditDialog"
            >
              Edit Profile
            </v-btn>
          </div>
          <p v-if="profile.email" class="user-email">{{ profile.email }}</p>
          <div class="user-joined">
            <v-icon icon="mdi-calendar-range" size="small" class="joined-icon" />
            <span>Joined {{ formatDate(profile.createdAt) }}</span>
          </div>
        </div>
      </section>

      <!-- Stats Section -->
      <section class="profile-stats">
        <v-row>
          <v-col v-for="stat in statCards" :key="stat.title" cols="12" sm="6" md="3">
            <v-card class="stat-card" variant="flat">
              <div class="stat-card-content">
                <v-avatar :color="stat.color" variant="tonal" class="stat-icon" rounded="lg">
                  <v-icon :icon="stat.icon" />
                </v-avatar>
                <div>
                  <div class="stat-value">{{ stat.value.toLocaleString() }}</div>
                  <div class="stat-label">
                    {{ stat.title }}
                  </div>
                </div>
              </div>
            </v-card>
          </v-col>
        </v-row>
      </section>
    </div>

    <div v-else class="not-found-state">
      <v-icon icon="mdi-account-off" size="100" class="not-found-icon" />
      <h2 class="not-found-title">User not found</h2>
      <v-btn color="primary" variant="tonal" rounded class="home-btn" @click="router.push('/')">
        Go Home
      </v-btn>
    </div>

    <!-- Edit Profile Dialog -->
    <v-dialog v-model="editDialog" max-width="500px">
      <v-card color="surface-container-high" rounded="xl">
        <v-card-title class="dialog-header">
          <span>Edit Profile</span>
          <v-btn
            icon="mdi-close"
            variant="text"
            @click="editDialog = false"
            :disabled="saving"
          ></v-btn>
        </v-card-title>

        <v-divider></v-divider>

        <v-card-text class="dialog-body">
          <div class="avatar-preview-wrapper">
            <user-avatar
              :name="editName || (profile ? profile.name : '')"
              :avatar-id="editAvatar || (profile ? profile.avatarId : null)"
              :size="180"
              elevation="2"
            />
            <v-btn
              v-if="!showExplanation"
              class="avatar-edit-overlay"
              color="transparent"
              @click="showExplanation = true"
            >
              <v-icon class="avatar-edit-icon" size="40" color="white">mdi-pencil</v-icon>
            </v-btn>
          </div>

          <v-expand-transition mode="out-in">
            <v-card
              rounded="xl"
              variant="tonal"
              color="primary"
              v-if="showExplanation"
              class="pa-4 mt-5"
            >
              <v-card-title>Edit picture</v-card-title>
              <v-divider />
              <v-card-text>
                To edit your profile picture, select one image, and click 'Set as profile picture'
                in the selection overlay. Or you can have the server automatically find a profile
                picture.
              </v-card-text>
              <v-card-actions>
                <v-spacer />
                <v-btn
                  class="px-5"
                  rounded
                  variant="tonal"
                  prepend-icon="mdi-auto-fix"
                  @click="autoSetProfilePic"
                  >Auto-set</v-btn
                >
                <v-btn
                  class="px-5"
                  exact
                  rounded
                  variant="tonal"
                  prepend-icon="mdi-chevron-right"
                  to="/"
                  >Photos</v-btn
                >
              </v-card-actions>
            </v-card>
          </v-expand-transition>

          <v-text-field
            v-model="editName"
            label="Name"
            variant="outlined"
            base-color="outline"
            autofocus
            class="mt-5"
            rounded
            prepend-inner-icon="mdi-account"
            :disabled="saving"
            hide-details
            @keyup.enter="saveProfile"
          ></v-text-field>
        </v-card-text>

        <v-card-actions class="pa-4">
          <v-spacer></v-spacer>
          <v-btn variant="text" rounded @click="editDialog = false" :disabled="saving">
            Cancel
          </v-btn>
          <v-btn color="primary" variant="tonal" rounded :loading="saving" @click="saveProfile">
            Save Changes
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </main-layout-container>
</template>

<style scoped>
.profile-view {
  padding: 40px;
}

.profile-content {
  max-width: 1200px;
  margin: 0 auto;
}

/* State Containers */
.loading-state,
.not-found-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 400px;
}

/* Header Section */
.profile-header {
  display: flex;
  margin-bottom: 48px;
  gap: 32px;
}

.user-info {
  flex-grow: 1;
}

.user-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.user-name {
  font-size: 3.5rem;
  font-weight: 700;
  margin: 0;
  line-height: 1.2;
}

.edit-btn {
  text-transform: none;
}

.user-email {
  font-size: 1.125rem;
  opacity: 0.7;
  margin-bottom: 8px;
}

.user-joined {
  display: flex;
  align-items: center;
  opacity: 0.6;
  font-size: 0.875rem;
}

.joined-icon {
  margin-right: 8px;
}

.stat-card {
  padding: 24px;
  border-radius: 24px;
  transition:
    transform 0.3s ease,
    box-shadow 0.3s ease;
  background: rgba(var(--v-theme-surface-variant), 0.3);
}

.stat-card-content {
  display: flex;
  align-items: center;
}

.stat-icon {
  margin-right: 16px;
}

.stat-value {
  font-size: 1.75rem;
  font-weight: 700;
  line-height: 1.2;
}

.stat-label {
  font-size: 0.75rem;
  text-transform: uppercase;
  font-weight: 600;
  opacity: 0.6;
  letter-spacing: 0.5px;
}

/* Not Found State */
.not-found-icon {
  opacity: 0.2;
  margin-bottom: 16px;
}

.not-found-title {
  font-size: 2rem;
  font-weight: 500;
}

.home-btn {
  margin-top: 24px;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px !important;
}

.dialog-body {
  padding: 32px 24px !important;
}

.avatar-preview-wrapper {
  display: flex;
  justify-content: center;
  position: relative;
}

.avatar-edit-overlay {
  width: 180px;
  height: 180px;
  border-radius: 50%;
  position: absolute;
  transition: background-color 0.15s;
  display: flex;
  place-items: center;
  place-content: center;
}

.avatar-edit-overlay:hover {
  background-color: rgba(0, 0, 0, 0.4);
}

.avatar-edit-icon {
  opacity: 0;
}

.avatar-edit-overlay:hover .avatar-edit-icon {
  opacity: 0.8;
}

.avatar-edit-buttons {
  display: flex;
  gap: 10px;
}

/* Mobile Adjustments */
@media (max-width: 600px) {
  .profile-header {
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .user-title-row {
    flex-direction: column;
    gap: 16px;
  }

  .user-name {
    font-size: 2.5rem;
  }

  .user-joined {
    justify-content: center;
  }
}
</style>
