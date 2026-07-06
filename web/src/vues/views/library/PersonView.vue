<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useTheme } from 'vuetify/framework'
import { usePeopleStore } from '@/scripts/stores/peopleStore.ts'
// eslint-disable-next-line
import SimpleTimeline from '@/vues/components/timeline/simple-timeline/SimpleTimeline.vue'
import peopleService from '@/scripts/services/peopleService.ts'
import PersonNameDialog from '@/vues/components/rename-people/PersonNameDialog.vue'

const theme = useTheme()
const route = useRoute()
const peopleStore = usePeopleStore()
const simpleTimeline = useTemplateRef('simpleTimeline')

const isInitialLoad = ref(true)
const fetched = ref(false)
const nameDialogVisible = ref(false)

function openNameDialog() {
  nameDialogVisible.value = true
}

const personId = computed(() => {
  const rawId = route.params.personId
  return Array.isArray(rawId) ? rawId[0] : rawId
})

const personResponse = computed(() => {
  if (!personId.value) return null
  return peopleStore.personMedia.get(personId.value) ?? null
})
const person = computed(() => personResponse.value?.person ?? null)
const items = computed(() => personResponse.value?.items ?? [])

function photoCountText(count: number) {
  return `${count.toLocaleString()} photo${count === 1 ? '' : 's'}`
}

async function unmerge() {
  if (!personId.value) return
  await peopleStore.unmergePerson(personId.value)
}

async function setMainPhoto(clusterId: string) {
  if (!personId.value || !person.value || person.value.faceThumbId === clusterId) return
  await peopleStore.updatePerson(personId.value, { faceThumbId: clusterId })
}

watch(
  personId,
  () => {
    isInitialLoad.value = true
    simpleTimeline.value?.scrollToTop()
    if (!personId.value) return
    peopleStore.fetchPeople()
    peopleStore.fetchPersonMedia(personId.value).then(() => (fetched.value = true))
  },
  { immediate: true },
)

watch(personResponse, () => {
  if (personResponse.value !== null) isInitialLoad.value = false
})
</script>

<template>
  <div class="person-page">
    <simple-timeline
      ref="simpleTimeline"
      v-if="personId"
      :timeline-items="items"
      :view-link="`/person/${personId}/view/`"
    >
      <div class="person-header" v-if="fetched">
        <div class="person-header-left">
          <v-avatar class="person-avatar" size="176">
            <img
              :src="peopleStore.getPhotoThumb(person, theme.current.value.dark)"
              alt="Image of person"
              v-if="person"
            />
            <img :src="peopleService.getPersonThumbnail(personId)" alt="Image of person" v-else />
          </v-avatar>
        </div>
        <div class="person-header-right">
          <div class="person-title-row">
            <h1
              :class="{ unnamed: !person?.name, clickable: !person?.name }"
              @click="!person?.name ? openNameDialog() : undefined"
            >
              {{ person?.name || 'Unnamed' }}
            </h1>
            <v-btn
              icon="mdi-pencil"
              variant="tonal"
              color="primary"
              density="comfortable"
              v-tooltip:top="person?.name ? 'Edit name' : 'Set name'"
              @click="openNameDialog"
            />
          </div>
          <p class="person-meta" v-if="person">
            <span>{{ photoCountText(person.photoCount) }}</span>
          </p>

          <div class="subclusters" v-if="person && person.faceClusterIds.length > 1">
            <v-avatar
              v-for="clusterId in person.faceClusterIds"
              :key="clusterId"
              size="42"
              class="subcluster-avatar"
              :class="{ 'avatar-pointer': person.faceThumbId !== clusterId }"
              v-tooltip="{
                location: 'top',
                text: 'Use as main photo',
                disabled: person.faceThumbId === clusterId,
              }"
              v-ripple="person.faceThumbId !== clusterId"
              @click="person.faceThumbId === clusterId ? undefined : setMainPhoto(clusterId)"
            >
              <img
                :style="{ pointerEvents: 'none' }"
                :src="peopleService.getFaceThumbnail(clusterId)"
                alt=""
              />
            </v-avatar>
            <v-btn
              size="42"
              v-if="person && person.faceClusterIds.length > 1"
              icon="mdi-account-multiple-remove"
              variant="tonal"
              color="tertiary"
              density="comfortable"
              v-tooltip:top="'Separate merged people'"
              @click="unmerge"
            />
          </div>
        </div>
      </div>
      <div v-else class="loading-header">
        <v-lazy>
          <div class="center-loading">
            <v-progress-circular color="primary" indeterminate size="70" />
            <h2>Loading...</h2>
          </div>
        </v-lazy>
      </div>

      <div class="empty-person" v-if="items.length === 0 && !isInitialLoad">
        <v-icon color="on-surface-variant" size="170" icon="mdi-image-search-outline" />
        <h2>No photos found</h2>
      </div>
    </simple-timeline>

    <person-name-dialog
      v-model="nameDialogVisible"
      :person="person"
      @saved="peopleStore.fetchPeople"
    />
  </div>
</template>

<style scoped>
.person-page {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
}

.person-header {
  display: flex;
  width: 100%;
  margin-bottom: 12px;
}

.loading-header {
  height: 196px;
  width: 100%;
  margin-bottom: 12px;
  display: flex;
  place-content: center;
  place-items: center;
}

.center-loading {
  text-align: center;
}

.center-loading h2 {
  font-weight: 500;
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 20px;
  margin-top: 10px;
}

.person-header-left {
  padding: 10px;
}

.person-avatar {
  background-color: rgba(var(--v-theme-on-background), 0.08);
}

.person-avatar img,
.subcluster-avatar img,
.person-suggestion img,
.merge-dialog-body img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.person-header-right {
  min-width: 0;
  flex-grow: 1;
  padding: 20px;
}

.person-title-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}

.person-title-row h1 {
  min-width: 0;
  font-size: 50px;
  line-height: 1.2;
  font-weight: 500;
  margin: 0;
  overflow-wrap: anywhere;
}

.person-title-row h1.unnamed {
  color: rgb(var(--v-theme-on-surface-variant));
  font-style: italic;
  font-weight: 400;
}

.person-title-row h1.clickable {
  cursor: pointer;
}

.person-title-row h1.clickable:hover {
  color: rgb(var(--v-theme-primary));
}

.person-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 0;
  color: rgb(var(--v-theme-on-surface-variant));
}

.subclusters {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 18px;
  align-items: center;
}

.subcluster-avatar {
  background-color: rgba(var(--v-theme-on-background), 0.08);
  box-shadow: 0 0 0 2px rgb(var(--v-theme-background));
}

.avatar-pointer {
  cursor: pointer;
}

.dialog-title {
  display: flex;
  align-items: center;
  background-color: rgb(var(--v-theme-surface-variant));
  font-size: 1.25rem;
  font-weight: 500;
  line-height: 2rem;
  padding: 12px 16px;
}

.dialog-title-icon {
  margin-right: 12px;
}

.dialog-content {
  padding: 24px;
}

.dialog-actions {
  padding: 12px;
}

.name-dialog:deep(.v-field__input) {
  min-height: 48px;
}

.name-dialog:deep(.v-field),
.name-dialog:deep(.v-field__input),
.name-dialog:deep(input) {
  cursor: text;
}

.person-suggestion:deep(.v-list-item__content) {
  min-width: 0;
}

.merge-dialog-body {
  display: flex;
  align-items: center;
  gap: 18px;
}

.merge-dialog-body p {
  margin: 0;
}

.merge-subtitle {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.9rem;
  margin-top: 4px !important;
}

.empty-person {
  height: 560px;
  width: 100%;
  display: flex;
  place-items: center;
  place-content: center;
  flex-direction: column;
  color: rgb(var(--v-theme-on-surface-variant));
}

.empty-person h2 {
  font-weight: 500;
  margin: 0;
}

@media (max-width: 720px) {
  .person-header {
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .person-header-right {
    padding: 10px;
    width: 100%;
  }

  .person-title-row {
    justify-content: center;
    align-items: center;
    flex-direction: column;
  }

  .person-title-row h1 {
    font-size: 34px;
  }

  .person-meta,
  .subclusters {
    justify-content: center;
  }
}
</style>
