<script setup lang="ts">
import { computed, ref } from 'vue'
import { useTheme } from 'vuetify/framework'
import { usePeopleStore } from '@/scripts/stores/peopleStore'
import type { PersonInfo } from '@/scripts/types/generated/timeline'

const props = defineProps<{
  modelValue: boolean
  sourcePerson: PersonInfo | null // The person who will remain
  targetPerson: PersonInfo | null // The person who will be deleted
}>()

const emit = defineEmits(['update:modelValue', 'confirmed'])

const theme = useTheme()
const peopleStore = usePeopleStore()
const isMerging = ref(false)

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})

function photoCountText(count: number) {
  return `${count.toLocaleString()} photo${count === 1 ? '' : 's'}`
}

const resultName = computed(() => {
  return props.sourcePerson?.name?.trim() || props.targetPerson?.name?.trim() || 'Unnamed'
})

const resultPhotoCount = computed(() => {
  const sourceCount = props.sourcePerson?.photoCount || 0
  const targetCount = props.targetPerson?.photoCount || 0
  return sourceCount + targetCount
})

const sourcePhoto = computed(() => {
  if (!props.sourcePerson) return ''
  return peopleStore.getPhotoThumb(props.sourcePerson, theme.current.value.dark)
})

const targetPhoto = computed(() => {
  if (!props.targetPerson) return ''
  return peopleStore.getPhotoThumb(props.targetPerson, theme.current.value.dark)
})

const resultPhoto = computed(() => {
  // If source has a explicitly set thumb or a generic thumb, use sourcePhoto.
  // Actually, backend uses COALESCE(source.face_thumb_id, target.face_thumb_id)
  // If source has faceThumbId, it stays. If not, it falls back to target's faceThumbId (or source's cluster).
  // For the frontend visualization, if source has a faceThumbId, use source.
  // Otherwise, maybe just use sourcePhoto as it's the first cluster.
  if (props.sourcePerson?.faceThumbId) return sourcePhoto.value
  if (props.targetPerson?.faceThumbId) return targetPhoto.value
  return sourcePhoto.value
})

function cancel() {
  visible.value = false
}

async function confirm() {
  if (!props.sourcePerson || !props.targetPerson || isMerging.value) return
  isMerging.value = true
  const success = await peopleStore.mergePerson(props.sourcePerson.id, props.targetPerson.id)
  isMerging.value = false
  if (success) {
    visible.value = false
    emit('confirmed')
  }
}
</script>

<template>
  <v-dialog v-model="visible" max-width="520" persistent>
    <v-card rounded="xl" color="surface-container" class="merge-dialog">
      <v-card-title class="dialog-title">
        <v-icon icon="mdi-account-multiple-plus" class="dialog-title-icon" />
        <span>Are these the same person?</span>
      </v-card-title>

      <v-card-text class="dialog-content" v-if="sourcePerson && targetPerson">
        <p class="explanation">They can be merged together</p>
        <div class="merge-visualization">
          <div class="merge-row">
            <!-- Target (Deleted) -->
            <div class="person-info target">
              <v-avatar size="72" class="mb-1 person-avatar">
                <img :src="targetPhoto" alt="" />
              </v-avatar>
              <div class="count">{{ photoCountText(targetPerson.photoCount) }}</div>
            </div>

            <!-- Arrow -->
            <v-icon icon="mdi-arrow-right" size="36" color="primary" />

            <!-- Source (Remains) -->
            <div class="person-info source">
              <v-avatar size="72" class="mb-1 person-avatar">
                <img :src="sourcePhoto" alt="" />
              </v-avatar>
              <div class="count">{{ photoCountText(sourcePerson.photoCount) }}</div>
            </div>
          </div>

          <v-divider class="my-6" />

          <div class="merge-result text-center">
            <div class="text-subtitle-2 text-medium-emphasis mb-4 text-uppercase tracking-wider">
              Merged
            </div>
            <v-avatar size="110" class="mb-3 result-avatar person-avatar">
              <img :src="resultPhoto" alt="" />
            </v-avatar>
            <div class="text-h6" :class="{ unnamed: resultName === 'Unnamed' }">
              {{ resultName }}
            </div>
            <div class="photo-count">
              {{ photoCountText(resultPhotoCount) }}
            </div>
          </div>
        </div>
      </v-card-text>

      <v-divider />

      <v-card-actions class="dialog-actions">
        <v-spacer />
        <v-btn variant="text" rounded="lg" @click="cancel">No, keep separate</v-btn>
        <v-btn color="primary" variant="tonal" rounded="lg" :loading="isMerging" @click="confirm">
          Merge
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
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
  padding: 32px 24px;
}

.dialog-actions {
  padding: 12px;
}

.explanation {
  color: rgb(var(--v-theme-on-surface));
  margin-bottom: 35px;
  text-align: center;
}

.merge-visualization {
  display: flex;
  flex-direction: column;
}

.merge-row {
  display: flex;
  align-items: center;
  justify-content: center;
}

.person-info {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 140px;
  text-align: center;
}

.person-info .name,
.merge-result .text-h6 {
  font-weight: 600;
  width: 100%;
}

.person-info .name.unnamed,
.merge-result .text-h6.unnamed {
  color: rgb(var(--v-theme-on-surface-variant));
  font-style: italic;
  font-weight: 400;
}

.person-info .count {
  color: rgb(var(--v-theme-on-surface-variant));
  font-size: 0.85rem;
  margin-top: 4px;
}

.person-avatar {
  background-color: rgba(var(--v-theme-on-background), 0.08);
}

.person-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.photo-count {
  color: rgb(var(--v-theme-on-surface-variant));
}

.tracking-wider {
  letter-spacing: 0.05em !important;
}
</style>
