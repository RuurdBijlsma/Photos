<script setup lang="ts">
import GlowImage from '@/vues/components/ui/GlowImage.vue'
import { computed, ref } from 'vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { getThumbnailHeight } from '@/scripts/utils.ts'
import { useTheme } from 'vuetify/framework'

const theme = useTheme()

const props = withDefaults(
  defineProps<{
    mediaItemId: string | null
    height?: number
    width?: number
    maxWidth?: number
    borderRadius?: string
    strength?: number
  }>(),
  {
    borderRadius: '0',
    strength: 1,
  },
)
const useOnDemandThumb = ref(false)
const thumbHeight = computed(() => getThumbnailHeight(props.height ?? 480))
const primaryThumb = computed(() => {
  if (props.mediaItemId === null) {
    if (theme.current.value.dark) return 'img/album-no-thumb-dark.png'
    else return 'img/album-no-thumb.png'
  }
  return mediaItemService.getPhotoThumbnail(
    props.mediaItemId,
    thumbHeight.value,
    useOnDemandThumb.value,
  )
})
</script>

<template>
  <glow-image
    :src="primaryThumb"
    :height="height"
    :width="width"
    :max-width="maxWidth"
    :border-radius="borderRadius"
    :strength="strength"
    @error="useOnDemandThumb = true"
  />
</template>

<style scoped></style>
