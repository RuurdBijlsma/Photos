<script setup lang="ts">
import MainLayoutContainer from '@/vues/components/MainLayoutContainer.vue'
import { computed, ref } from 'vue'
import { usePeopleStore } from '@/scripts/stores/peopleStore.ts'
import { useTheme } from 'vuetify/framework'
import GlowImage from '@/vues/components/ui/GlowImage.vue'
import type { PersonInfo } from '@/scripts/types/generated/timeline.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import PersonNameDialog from '@/vues/components/rename-people/PersonNameDialog.vue'
import MergePersonDialog from '@/vues/components/rename-people/MergePersonDialog.vue'

const theme = useTheme()
const peopleStore = usePeopleStore()
const dialogs = useDialogStore()
const draggedPersonId = ref<string | null>(null)
const dragOverId = ref<string | null>(null)
const dragCounters = ref<Record<string, number>>({})
// Rename functionality
const nameDialogVisible = ref(false)
const selectedPerson = ref<PersonInfo | null>(null)

// Merge functionality
const mergeDialogVisible = ref(false)
const mergeDialogSource = ref<PersonInfo | null>(null)
const mergeDialogTarget = ref<PersonInfo | null>(null)

const namedPeople = computed(() =>
  peopleStore.people
    .filter((person) => person.name?.trim())
    .sort((a, b) => b.photoCount - a.photoCount),
)
const unnamedPeople = computed(() =>
  peopleStore.people
    .filter((person) => !person.name?.trim())
    .sort((a, b) => b.photoCount - a.photoCount),
)

const sections = computed(() =>
  [
    { title: 'Named', items: namedPeople.value },
    { title: 'Unnamed', items: unnamedPeople.value },
  ].filter((section) => section.items.length > 0),
)

const totalPeopleCount = computed(() => peopleStore.people.length)

function setName(e: PointerEvent, person: PersonInfo) {
  e.preventDefault()
  e.stopPropagation()
  selectedPerson.value = person
  nameDialogVisible.value = true
}

function photoCountText(count: number) {
  return `${count.toLocaleString()} photo${count === 1 ? '' : 's'}`
}

function onDragStart(person: PersonInfo, event: DragEvent) {
  draggedPersonId.value = person.id
  dragCounters.value = {}
  event.dataTransfer?.setData('text/plain', person.id)
  if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move'
}

function onDragEnd() {
  draggedPersonId.value = null
  dragOverId.value = null
  dragCounters.value = {}
}

function handleDragEnter(personId: string) {
  if (draggedPersonId.value === personId) return

  // Increment counter for this specific card
  dragCounters.value[personId] = (dragCounters.value[personId] || 0) + 1
  dragOverId.value = personId
}

function handleDragLeave(personId: string) {
  if (draggedPersonId.value === personId) return

  // Decrement counter
  dragCounters.value[personId] = (dragCounters.value[personId] || 0) - 1

  // Only remove highlight if we've truly left the parent and all its children
  if (dragCounters.value[personId] <= 0) {
    if (dragOverId.value === personId) {
      dragOverId.value = null
    }
  }
}

async function onDrop(dropPerson: PersonInfo, event: DragEvent) {
  const sourceId = event.dataTransfer?.getData('text/plain') || draggedPersonId.value
  draggedPersonId.value = null
  dragOverId.value = null
  dragCounters.value = {}
  if (!sourceId || sourceId === dropPerson.id) return

  const sourcePerson = peopleStore.people.find((person) => person.id === sourceId)
  if (!sourcePerson) return

  // The person being dropped ON is the one who remains (source)
  // The person being dragged is the one who is deleted (target)
  const mergeSource = dropPerson
  const mergeTarget = sourcePerson

  const sourceName = mergeSource.name?.trim()
  const targetName = mergeTarget.name?.trim()

  if (!sourceName && !targetName) {
    const name = await dialogs.prompt({
      title: 'Name merged person',
      description: 'Give this person a name before merging these two groups.',
      confirmText: 'Merge',
    })
    if (!name?.trim()) return
    const updated = await peopleStore.updatePerson(mergeSource.id, { name: name.trim() })
    if (!updated) return

    await peopleStore.mergePerson(mergeSource.id, mergeTarget.id)
    return
  }

  mergeDialogSource.value = mergeSource
  mergeDialogTarget.value = mergeTarget
  mergeDialogVisible.value = true
}

function onMergeConfirmed() {
  mergeDialogSource.value = null
  mergeDialogTarget.value = null
}

peopleStore.fetchPeople()
</script>

<template>
  <main-layout-container>
    <person-name-dialog v-model="nameDialogVisible" :person="selectedPerson" />
    <merge-person-dialog
      v-model="mergeDialogVisible"
      :source-person="mergeDialogSource"
      :target-person="mergeDialogTarget"
      @confirmed="onMergeConfirmed"
    />
    <div class="library-container">
      <header class="library-header">
        <div class="header-left">
          <h1>People</h1>
          <span class="people-count">
            {{ totalPeopleCount.toLocaleString() }} person{{ totalPeopleCount === 1 ? '' : 's' }}
          </span>
        </div>
      </header>

      <template v-if="totalPeopleCount > 0">
        <section v-for="section in sections" :key="section.title" class="people-section">
          <div class="section-header">
            <h2>{{ section.title }}</h2>
            <p>{{ section.items.length.toLocaleString() }}</p>
          </div>

          <div class="people-grid">
            <router-link
              v-for="person in section.items"
              :key="person.id"
              :to="`/person/${person.id}`"
              class="person-card"
              draggable="true"
              @dragstart="onDragStart(person, $event)"
              @dragend="onDragEnd"
              :class="{
                'drag-over-card': dragOverId === person.id,
                'is-dragging': draggedPersonId !== null,
              }"
              @dragenter="handleDragEnter(person.id)"
              @dragleave="handleDragLeave(person.id)"
              @dragover.prevent
              @drop.prevent="onDrop(person, $event)"
              @mouseenter="peopleStore.fetchPersonMedia(person.id, true, false)"
            >
              <glow-image
                :src="peopleStore.getPhotoThumb(person, theme.current.value.dark)"
                :height="124"
                :width="124"
                border-radius="62px"
                :strength="0.7"
              />
              <div class="person-copy">
                <p
                  v-if="!person.name?.trim()"
                  class="person-name unnamed"
                  v-ripple
                  @click="(e) => setName(e, person)"
                >
                  Unnamed
                </p>
                <p class="person-name" v-else>
                  {{ person.name?.trim() }}
                </p>
                <p class="person-count">{{ photoCountText(person.photoCount) }}</p>
              </div>
            </router-link>
          </div>
        </section>
      </template>

      <div class="empty-people" v-else>
        <v-icon color="on-surface-variant" size="140" icon="mdi-account-search-outline" />
        <h2>No people found</h2>
      </div>
    </div>
  </main-layout-container>
</template>

<style scoped>
.library-container {
  padding: 20px;
}

.library-header {
  padding: 0 10px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 30px;
}

.header-left h1 {
  margin: 0;
  font-size: 2.5rem;
  font-weight: 600;
  line-height: 1.2;
}

.people-count {
  font-size: 0.9rem;
  font-weight: 400;
  color: rgb(var(--v-theme-on-surface-variant));
}

.people-section {
  margin-bottom: 42px;
}

.section-header {
  display: flex;
  align-items: baseline;
  gap: 15px;
  margin-bottom: 14px;
  padding: 0 10px;
}

.section-header h2 {
  font-size: 24px;
  font-weight: 500;
  margin: 0;
}

.section-header p {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 20px;
  margin: 0;
}

.people-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
  gap: 24px 14px;
  justify-items: center;
}

.person-card {
  min-width: 0;
  border-radius: 8px;
  color: rgb(var(--v-theme-on-background));
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 10px;
  text-align: center;
  text-decoration: none;
  transition:
    background-color 150ms ease,
    transform 50ms ease,
    opacity 150ms ease;
}

.person-card:hover:not(:has(.unnamed:hover)) {
  background-color: rgba(var(--v-theme-on-background), 0.06);
  transform: translateY(-2px);
}

.person-card :deep(img) {
  pointer-events: none;
}

.person-card.drag-over-card {
  outline: 2px solid rgb(var(--v-theme-primary)) !important;
  background-color: rgba(var(--v-theme-primary), 0.1) !important;
  transform: scale(1.01);
}

.person-card[draggable='true']:active {
  opacity: 0.4;
}

.person-copy {
  min-width: 0;
  width: 100%;
}

.person-name,
.person-count {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.person-name {
  font-weight: 600;
  margin: 0;
}

.person-name.unnamed {
  color: rgb(var(--v-theme-on-surface-variant));
  font-style: italic;
  font-weight: 400;
  border-radius: 8px;
  transition: background-color 0.1s;
}

.person-name.unnamed:hover {
  background-color: rgba(var(--v-theme-on-surface-variant), 0.1);
}

.person-count {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.9rem;
  margin: 3px 0 0;
}

.empty-people {
  min-height: 520px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  color: rgb(var(--v-theme-on-surface-variant));
}

.empty-people h2 {
  font-weight: 500;
  margin: 0;
}
</style>
