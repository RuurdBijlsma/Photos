import { defineStore } from 'pinia'
import { computed, ref, shallowRef, triggerRef } from 'vue'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'

export const useSelectionStore = defineStore('selection', () => {
  const allIds = shallowRef<string[]>([])
  const selection = shallowRef(new Set<string>())
  const isSelecting = computed(() => selection.value.size > 0)
  const hoverDate = ref<string | null>(null)

  const dialogs = useDialogStore()

  let anchorId: string | null = null

  function toggleSelection(id: string) {
    if (selection.value.has(id)) selection.value.delete(id)
    else selection.value.add(id)
    anchorId = id
    triggerRef(selection)
  }

  async function selectAll() {
    if (selection.value.size > 10) {
      const confirmed = await dialogs.confirm({
        title: 'Change Selection?',
        description: 'Your current selection will be cleared and replaced with all items.',
        confirmText: 'Select all',
      })
      if (!confirmed) return
    }
    selection.value = new Set(allIds.value)
    anchorId = null
  }

  async function deselectAll(ignorePrompt = false) {
    if (
      selection.value.size > 10 &&
      allIds.value.length - selection.value.size > 10 &&
      !ignorePrompt
    ) {
      const confirmed = await dialogs.confirm({
        title: 'Clear Selection?',
        description: 'This will remove all selected items. This action cannot be undone.',
        confirmText: 'Clear selection',
      })
      if (!confirmed) return
    }
    selection.value = new Set()
    anchorId = null
  }

  function deselectMany(ids: string[]) {
    for (const id of ids) {
      selection.value.delete(id)
      if (anchorId == id) anchorId = null
    }
    triggerRef(selection)
  }

  function selectSpan(id: string) {
    if (anchorId === null) return toggleSelection(id)

    let indexA = allIds.value.indexOf(id)
    let indexB = allIds.value.indexOf(anchorId)
    if (indexA > indexB) {
      ;[indexA, indexB] = [indexB, indexA]
    }
    anchorId = id
    const selectionSlice = allIds.value.slice(indexA, indexB + 1)
    let allSelected = true
    for (const spanId of selectionSlice) {
      if (!selection.value.has(spanId)) {
        allSelected = false
        break
      }
    }
    if (!allSelected)
      for (const spanId of selectionSlice) {
        selection.value.add(spanId)
      }
    else {
      for (const spanId of selectionSlice) {
        selection.value.delete(spanId)
      }
      selection.value.add(id)
    }
    triggerRef(selection)
  }

  return {
    allIds,
    selection,
    isSelecting,
    hoverDate,

    deselectMany,
    toggleSelection,
    selectAll,
    deselectAll,
    selectSpan,
  }
})
