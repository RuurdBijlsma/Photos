import { defineStore } from 'pinia'
import type {
  CardCollectionPayload,
  CollectionMediaItem,
  DailyCardResponse,
} from '@/scripts/types/api/dailyCards.ts'
import { computed, ref } from 'vue'
import type { AxiosResponse } from 'axios'
import dailyCardService from '@/scripts/services/dailyCardService.ts'
import { useObjStorage } from '@/scripts/utils.ts'

export const useDailyCardStore = defineStore('dailyCard', () => {
  const cardsByDate = useObjStorage<Record<string, DailyCardResponse[]>>('dailyCardsByDate', {})
  const cardsPromises = new Map<string, Promise<AxiosResponse<DailyCardResponse[]>>>()
  const cardsById = computed(() => {
    const allCards = Object.values(cardsByDate.value).flat()
    const result = new Map<number, DailyCardResponse>()
    for (const card of allCards) {
      result.set(card.id, card)
    }
    return result
  })
  const completedCards = useObjStorage<number[]>('dailyCompletedCards', [])

  const todayDate = ref(new Date().toISOString().substring(0, 10))
  const todayCards = computed(() => cardsByDate.value[todayDate.value])

  function getPayloadItems(card: DailyCardResponse): CollectionMediaItem[] {
    const payload = card.payload as unknown as CardCollectionPayload
    console.log('collection', payload)
    return payload.mediaItems
  }

  async function fetchDailyCards() {
    const today = new Date().toISOString().substring(0, 10)
    todayDate.value = today
    await fetchCardsForDate(today)
    requestIdleCallback(() => {
      const tomorrow = new Date()
      tomorrow.setDate(tomorrow.getDate() + 1)
      fetchCardsForDate(tomorrow.toISOString().substring(0, 10))
    })
  }

  async function fetchCardsForDate(date: string) {
    if (cardsPromises.has(date)) {
      await cardsPromises.get(date)
      return
    }
    if (date in cardsByDate.value) return

    const promise = dailyCardService.getCards(date)
    cardsPromises.set(date, promise)
    const result = await promise
    cardsPromises.delete(date)

    cardsByDate.value = {
      ...cardsByDate.value,
      [date]: result.data,
    }
  }

  function cleanOldCache() {
    // Delete old daily cards from cache
    const currentCards = cardsByDate.value
    const keys = Object.keys(currentCards)
    const keysToKeep = keys.filter((key) => key >= todayDate.value)

    if (keysToKeep.length < keys.length) {
      const updatedCards: Record<string, DailyCardResponse[]> = {}
      for (const key of keysToKeep) {
        updatedCards[key] = currentCards[key]
      }
      cardsByDate.value = updatedCards
    }
    // Delete memory of completed cards
    if (completedCards.value.length > 10) {
      completedCards.value.splice(0, completedCards.value.length - 10)
    }
  }

  requestIdleCallback(cleanOldCache)

  return {
    cardsByDate,
    todayDate,
    todayCards,
    getPayloadItems,
    cardsById,
    completedCards,

    fetchDailyCards,
  }
})
