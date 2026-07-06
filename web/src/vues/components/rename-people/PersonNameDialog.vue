<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useTheme } from 'vuetify/framework'
import { usePeopleStore } from '@/scripts/stores/peopleStore'
import type { PersonInfo } from '@/scripts/types/generated/timeline'
import MergePersonDialog from './MergePersonDialog.vue'
import { useRoute, useRouter } from 'vue-router'

const props = defineProps<{
  modelValue: boolean
  person: PersonInfo | null
}>()

const emit = defineEmits(['update:modelValue', 'saved'])

const theme = useTheme()
const peopleStore = usePeopleStore()
const route = useRoute()
const router = useRouter()

const nameDialogVisible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

const mergeDialogVisible = ref(false)
const draftName = ref<string | PersonInfo | null>('')
const isSavingName = ref(false)

const mergeSource = ref<PersonInfo | null>(null)
const mergeTarget = ref<PersonInfo | null>(null)

const namedPeople = computed(() => {
  return peopleStore.people
    .filter((p) => p.id !== props.person?.id && p.name?.trim())
    .sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))
})

// Reset local state when dialog opens
watch(
  () => props.modelValue,
  (newVal) => {
    if (newVal && props.person) {
      draftName.value = props.person.name ?? ''
      mergeSource.value = null
      mergeTarget.value = null
    }
  },
)

function photoCountText(count: number) {
  return `${count.toLocaleString()} photo${count === 1 ? '' : 's'}`
}

function getDraftName() {
  if (typeof draftName.value === 'string') return draftName.value.trim()
  return draftName.value?.name?.trim() ?? ''
}

function suggestionItemProps(props: Record<string, unknown>) {
  const itemProps = { ...props }
  delete itemProps.title
  delete itemProps.subtitle
  return itemProps
}

async function submitNameDialog() {
  if (!props.person || isSavingName.value) return

  const trimmedName = getDraftName()
  const currentName = props.person.name ?? ''

  // Check if chosen name matches an existing person
  const target = namedPeople.value.find(
    (p) => p.name?.trim().toLocaleLowerCase() === trimmedName.toLocaleLowerCase(),
  )

  const nextName = trimmedName.length > 0 ? trimmedName : null
  if ((nextName ?? '') === currentName) {
    nameDialogVisible.value = false
    return
  }

  isSavingName.value = true
  const updated = await peopleStore.updatePerson(props.person.id, { name: nextName })
  isSavingName.value = false

  if (target && trimmedName.length > 0) {
    if (props.person.photoCount > target.photoCount) {
      mergeSource.value = props.person
      mergeTarget.value = target
    } else {
      mergeSource.value = target
      mergeTarget.value = props.person
    }

    nameDialogVisible.value = false
    mergeDialogVisible.value = true
    return
  }

  if (updated) {
    nameDialogVisible.value = false
    emit('saved')
  }
}

async function onMergeConfirmed() {
  const targetId = mergeTarget.value?.id
  const sourceId = mergeSource.value?.id
  mergeDialogVisible.value = false
  mergeSource.value = null
  mergeTarget.value = null
  emit('saved')
  console.warn({
    targetId,
    rn: route.name,
    rpp: route.params.personId,
  })

  // If the user was viewing the person that just got deleted, redirect to the new person
  if (targetId && route.name === 'person-view' && route.params.personId === targetId) {
    console.warn('Y')
    await router.replace(`/person/${sourceId || ''}`)
  }
}
</script>

<template>
  <div>
    <!-- Primary Naming Dialog -->
    <v-dialog v-model="nameDialogVisible" max-width="520">
      <v-card rounded="xl" color="surface-container" class="name-dialog">
        <v-card-title class="dialog-title">
          <v-icon icon="mdi-account-edit" class="dialog-title-icon" />
          <span>Edit person name</span>
          <v-spacer />
          <v-btn
            icon="mdi-close"
            variant="text"
            density="comfortable"
            @click="nameDialogVisible = false"
          />
        </v-card-title>

        <v-card-text class="dialog-content">
          <v-combobox
            v-model="draftName"
            :items="namedPeople"
            item-title="name"
            item-value="name"
            label="Name"
            placeholder="Unnamed"
            name="person-name-edit"
            autocomplete="off"
            autofocus
            clearable
            color="primary"
            variant="outlined"
            hide-details
            @keydown.enter.prevent="submitNameDialog"
          >
            <template v-slot:item="{ props: itemProps, item }">
              <v-list-item v-bind="suggestionItemProps(itemProps)" class="person-suggestion">
                <template v-slot:prepend>
                  <v-avatar size="36">
                    <img :src="peopleStore.getPhotoThumb(item, theme.current.value.dark)" alt="" />
                  </v-avatar>
                </template>
                <v-list-item-title>{{ item.name }}</v-list-item-title>
                <v-list-item-subtitle>
                  {{ photoCountText(item.photoCount) }}
                </v-list-item-subtitle>
              </v-list-item>
            </template>
          </v-combobox>
        </v-card-text>

        <v-divider />

        <v-card-actions class="dialog-actions">
          <v-spacer />
          <v-btn variant="text" rounded="lg" @click="nameDialogVisible = false">Cancel</v-btn>
          <v-btn
            color="primary"
            variant="tonal"
            rounded="lg"
            :loading="isSavingName"
            @click="submitNameDialog"
          >
            Save
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <!-- Secondary Merge Confirmation Dialog -->
    <merge-person-dialog
      v-model="mergeDialogVisible"
      :source-person="mergeSource"
      :target-person="mergeTarget"
      @confirmed="onMergeConfirmed"
    />
  </div>
</template>

<style scoped>
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
.person-suggestion img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
