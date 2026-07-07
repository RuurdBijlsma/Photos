import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useRefreshStore = defineStore('refresh', () => {
  const counter = ref(0)

  return {
    counter,
  }
})
