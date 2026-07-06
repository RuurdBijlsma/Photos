import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export type DialogType = 'alert' | 'confirm' | 'prompt'

export interface DialogAction {
  name: string
  color?: string
  action: () => unknown
}

export interface DialogOptions {
  title: string
  description?: string
  icon?: string
  confirmText?: string
  cancelText?: string
  defaultValue?: string
  persistent?: boolean
  color?: string
  actions?: DialogAction[]
  attach?: boolean
}

interface DialogRequest {
  id: string
  type: DialogType
  options: DialogOptions
  resolve: (value: any) => void // eslint-disable-line @typescript-eslint/no-explicit-any
}

export const useDialogStore = defineStore('dialog', () => {
  const queue = ref<DialogRequest[]>([])
  const current = ref<DialogRequest | null>(null)
  const visible = ref(false)
  const inputValue = ref('')
  const customVisible = ref(false)
  const anyVisible = computed(() => customVisible.value || visible.value)

  // Track the result internally so we can resolve after the transition
  let pendingResult: any = null // eslint-disable-line @typescript-eslint/no-explicit-any

  function processQueue() {
    // If a dialog is already visible, don't start a new one.
    // The 'close' function will call processQueue again.
    if (visible.value || queue.value.length === 0) return

    current.value = queue.value[0]
    inputValue.value = current.value.options.defaultValue ?? ''
    visible.value = true
  }

  function alert(options: DialogOptions | string): Promise<void> {
    const opts = typeof options === 'string' ? { title: 'Alert', description: options } : options
    return new Promise((resolve) => {
      queue.value.push({ id: crypto.randomUUID(), type: 'alert', options: opts, resolve })
      processQueue()
    })
  }

  function confirm(options: DialogOptions | string): Promise<boolean> {
    const opts = typeof options === 'string' ? { title: options } : options
    return new Promise((resolve) => {
      queue.value.push({ id: crypto.randomUUID(), type: 'confirm', options: opts, resolve })
      processQueue()
    })
  }

  function prompt(options: DialogOptions | string): Promise<string | null> {
    const opts = typeof options === 'string' ? { title: options } : options
    return new Promise((resolve) => {
      queue.value.push({ id: crypto.randomUUID(), type: 'prompt', options: opts, resolve })
      processQueue()
    })
  }

  function handleConfirm() {
    if (!current.value) return
    pendingResult = current.value.type === 'prompt' ? inputValue.value : true
    close()
  }

  function handleCancel() {
    if (!current.value) return
    pendingResult = current.value.type === 'confirm' ? false : null
    close()
  }

  async function close() {
    visible.value = false

    // Wait for Vuetify's transition to finish (~200-300ms)
    // before resolving the promise and clearing the state.
    await new Promise((resolve) => setTimeout(resolve, 300))

    const resolvedRequest = queue.value.shift()
    current.value = null
    inputValue.value = ''

    // Resolve the promise only AFTER the UI is clear
    if (resolvedRequest) {
      resolvedRequest.resolve(pendingResult)
    }

    pendingResult = null

    // Check if there are more dialogs waiting in the queue
    processQueue()
  }

  return {
    visible,
    customVisible,
    anyVisible,
    inputValue,
    current,
    alert,
    confirm,
    prompt,
    handleConfirm,
    handleCancel,
  }
})
