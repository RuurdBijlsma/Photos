<script setup lang="ts">
import { computed } from 'vue'
import { getThumbnailHeight, stringToColor } from '@/scripts/utils.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'

const props = defineProps<{
  name: string
  avatarId?: string | null
  size?: string | number
}>()

const avatarSize = computed(() => Number(props.size) || 40)

const avatarUrl = computed(() => {
  if (!props.avatarId) return null
  return mediaItemService.getPhotoThumbnail(
    props.avatarId,
    getThumbnailHeight(avatarSize.value),
    false,
  )
})

const initials = computed(() => {
  if (!props.name) return '?'
  return props.name
    .split(' ')
    .map((i) => i[0]?.toUpperCase())
    .filter(Boolean)
    .join('')
    .slice(0, 2)
})

const backgroundColor = computed(() => stringToColor(props.name))
</script>

<template>
  <v-avatar
    :size="avatarSize"
    :style="{
      fontSize: avatarSize / 2.7 + 'px',
      fontWeight: avatarSize > 40 ? 'bold' : '600',
    }"
    :color="avatarId ? undefined : backgroundColor"
  >
    <v-img v-if="avatarUrl" :src="avatarUrl" cover />
    <template v-else>{{ initials }}</template>
  </v-avatar>
</template>
