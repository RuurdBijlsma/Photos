<script setup lang="ts">
import { computed, ref } from 'vue'
import ThumbnailImg from '@/vues/components/ui/ThumbnailImg.vue'
import type { FullMediaItem } from '@/scripts/types/api/fullPhoto.js'
import DateTimePicker from '@/vues/components/viewer/components/DateTimePicker.vue'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.js'
import { useDialogStore } from '@/scripts/stores/dialogStore.js'
import { formatNaiveDate } from '@/scripts/utils.js'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.js'
import { useTimelineStore } from '@/scripts/stores/timeline/timelineStore.js'

const props = defineProps<{
  mediaItem: FullMediaItem
}>()

const emit = defineEmits(['closeDialog'])
const mediaItemStore = useMediaItemStore()
const dialogs = useDialogStore()
const snackbars = useSnackbarsStore()
const timelineStore = useTimelineStore()

const adjustedDate = ref(new Date(props.mediaItem.taken_at_local))
const originalDate = new Date(props.mediaItem.og_taken_at_local)

const timezoneDisplay = computed(() => {
  const is_tz_edited =
    props.mediaItem.og_timezone_offset_seconds !== props.mediaItem.timezone_offset_seconds
  const tz = is_tz_edited
    ? makeTzOffsetString(props.mediaItem.timezone_offset_seconds)
    : props.mediaItem.timezone_name
  if (!tz) return 'the local timezone'
  return /^[+-]\d{2}:\d{2}$/.test(tz) ? `UTC${tz}` : tz
})
const ogTimezoneDisplay = computed(() => {
  const tz =
    props.mediaItem.timezone_name ?? makeTzOffsetString(props.mediaItem.og_timezone_offset_seconds)
  if (!tz) return 'the local timezone'
  return /^[+-]\d{2}:\d{2}$/.test(tz) ? `UTC${tz}` : tz
})

function revert() {
  adjustedDate.value = new Date(props.mediaItem.og_taken_at_local)
}

function makeTzOffsetString(tzOffsetSeconds: number | undefined | null) {
  if (tzOffsetSeconds == null) {
    return null
  }

  const sign = tzOffsetSeconds >= 0 ? '+' : '-'
  const absoluteSeconds = Math.abs(tzOffsetSeconds)
  const hours = Math.floor(absoluteSeconds / 3600)
  const minutes = Math.floor((absoluteSeconds % 3600) / 60)
  return `${sign}${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`
}

function parseTzOffsetString(value: string): number | null {
  const match = value.trim().match(/^([+-])(\d{2}):(\d{2})$/)
  if (!match) {
    return null
  }
  const [, sign, hoursString, minutesString] = match
  const hours = Number(hoursString)
  const minutes = Number(minutesString)
  if (hours > 23 || minutes > 59) {
    return null
  }
  return (hours * 3600 + minutes * 60) * (sign === '-' ? -1 : 1)
}

async function editTimezoneOffset() {
  const tzOffsetString = makeTzOffsetString(props.mediaItem.timezone_offset_seconds)
  const ogTzOffsetString = makeTzOffsetString(props.mediaItem.og_timezone_offset_seconds)

  const newTimezoneOffset = await dialogs.prompt({
    title: 'Timezone Offset',
    description: `
Set the ${props.mediaItem.is_video ? 'video' : 'photo'}'s UTC offset.

This should be the fixed timezone offset that applied when the photo was taken.

<br><br>

Current offset:
<code>${tzOffsetString ?? 'unknown'}</code>

<br>

Original offset:
<code>${ogTzOffsetString ?? 'unknown'}</code>

<br><br>

Enter the offset in the format <code>±HH:MM</code>.

<br>

Examples:
<ul>
  <li>The Netherlands (summer): <code>+02:00</code></li>
  <li>United Kingdom (winter): <code>+00:00</code></li>
  <li>New York (winter): <code>-05:00</code></li>
</ul>

Positive values are east of UTC.
Negative values are west of UTC.
`,
    defaultValue: tzOffsetString ?? undefined,
    confirmText: 'Save',
  })

  if (!newTimezoneOffset) return
  const parsedSeconds = parseTzOffsetString(newTimezoneOffset)
  if (parsedSeconds == null) {
    snackbars.error('Invalid timezone offset')
    return
  }

  await mediaItemStore.updateMediaItem(props.mediaItem.id, {
    timezoneOffsetSeconds: parsedSeconds,
  })
}

async function save() {
  if (!props.mediaItem) return
  const adjustedDateString = formatNaiveDate(adjustedDate.value)
  await mediaItemStore.updateMediaItem(props.mediaItem.id, {
    takenAtLocal: adjustedDateString,
  })
  requestIdleCallback(() => timelineStore.refresh())
  emit('closeDialog')
}
</script>

<template>
  <v-card class="edit-date-card pa-6" color="surface-container" rounded="xl">
    <v-card-title class="text-center mb-6"> Adjust Date and Time </v-card-title>

    <div class="d-flex flex-column align-center">
      <v-avatar size="110" rounded="lg" class="mb-5 elevation-3">
        <thumbnail-img cover :media-item-id="mediaItem?.id" />
      </v-avatar>

      <div class="tz-offset" @click="editTimezoneOffset()" v-ripple>
        <template v-if="mediaItem.timezone_offset_seconds !== mediaItem.og_timezone_offset_seconds">
          <!-- Edited timezone -->
          <p>Original time is {{ ogTimezoneDisplay }}</p>
          <p>Adjusted time is {{ timezoneDisplay }}</p>
        </template>
        <p v-else>All times are listed in {{ ogTimezoneDisplay }}</p>
      </div>

      <div class="time-row mb-3 mt-2">
        <span class="dt-label text-on-surface-variant">Original:</span>
        <date-time-picker disabled v-model="originalDate" />
      </div>

      <div class="time-row mb-2">
        <span class="dt-label text-on-surface-variant">Adjusted:</span>
        <date-time-picker v-model="adjustedDate" />
      </div>

      <v-btn variant="plain" color="primary" rounded="pill" @click="revert">
        Revert to Original
      </v-btn>
    </div>

    <v-card-actions class="mt-7">
      <v-btn variant="text" rounded @click="emit('closeDialog')"> Cancel </v-btn>
      <v-btn variant="tonal" color="primary" rounded @click="save"> Save </v-btn>
    </v-card-actions>
  </v-card>
</template>

<style scoped>
.edit-date-card {
  background-color: rgba(var(--v-theme-surface-container), 0.8) !important;
  backdrop-filter: saturate(250%) blur(12px) !important;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.time-row {
  display: flex;
  align-items: center;
  width: 100%;
  max-width: 380px;
}

.tz-offset {
  font-size: 14px;
  font-weight: 500;
  opacity: 0.7;
  cursor: pointer;
  padding: 3px 10px;
  border-radius: 20px;
  text-align: center;
}

.dt-label {
  font-weight: 500;
  font-size: 14px;
  margin-right: 10px;
  width: 75px;
}
</style>
