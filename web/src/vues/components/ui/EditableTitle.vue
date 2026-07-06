<script setup lang="ts">
import { computed, watch, nextTick, onMounted } from 'vue'
import { useTextareaAutosize } from '@vueuse/core'

const props = defineProps<{
  name: string
  autofocus: boolean
  modelValue: string
}>()
const emit = defineEmits(['update:modelValue', 'submit'])

const value = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val),
})
const { textarea, triggerResize } = useTextareaAutosize({ input: value })

function onEnter(e: KeyboardEvent) {
  emit('submit')
  const target = e.target as HTMLElement
  target.blur()
}

watch(value, () => {
  nextTick(() => triggerResize())
})
watch(
  [() => props.autofocus, textarea],
  () => {
    const el = textarea.value
    if (el) {
      if (props.autofocus) {
        el.focus()
        el.select()
      }
    }
  },
  { immediate: true },
)
onMounted(() => {
  triggerResize()
})
</script>

<template>
  <textarea
    :name="name"
    ref="textarea"
    class="editable-title"
    v-model="value"
    rows="1"
    placeholder="Unnamed"
    spellcheck="false"
    @input="triggerResize"
    @keydown.enter.prevent="onEnter"
  />
</template>

<style scoped>
.editable-title {
  background: transparent;
  border: none;
  outline: none;
  margin: 0;
  resize: none;
  overflow: hidden;
  font-family: inherit;
  font-weight: 500;
  font-size: 50px;
  line-height: 1.2;
  color: rgb(var(--v-theme-on-background));
  width: 100%;
  display: block;
  padding: 5px 15px;
  border-radius: 20px;
  transition: background-color 0.2s;
  box-sizing: border-box;
}

.editable-title:hover {
  background-color: rgba(var(--v-theme-on-background), 0.05);
}

.editable-title:focus {
  background-color: rgba(var(--v-theme-on-background), 0.1);
}

.editable-title::placeholder {
  font-style: italic;
  opacity: 0.5;
  color: inherit;
}
</style>
