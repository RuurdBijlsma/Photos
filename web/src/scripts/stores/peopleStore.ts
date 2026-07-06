import { defineStore } from 'pinia'
import { shallowRef, triggerRef } from 'vue'
import type { FullPersonMediaResponse, PersonInfo } from '@/scripts/types/generated/timeline.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import peopleService from '@/scripts/services/peopleService.ts'
import { useDialogStore } from '@/scripts/stores/dialogStore.ts'
import { useRouter } from 'vue-router'
import type { UpdatePersonRequest } from '@/scripts/types/api/people.ts'
import { useObjStorage } from '@/scripts/utils.ts'

export const usePeopleStore = defineStore('people', () => {
  const snackbarStore = useSnackbarsStore()
  const dialogs = useDialogStore()
  const router = useRouter()

  const people = useObjStorage<PersonInfo[]>('userPeople', [])
  const personMedia = shallowRef(new Map<string, FullPersonMediaResponse>())
  const personMediaPromises = new Map<string, Promise<FullPersonMediaResponse>>()

  async function fetchPeople() {
    try {
      const response = await peopleService.list()
      console.log('People list', response.people)
      people.value = response.people
    } catch (e) {
      snackbarStore.error("Can't fetch people", e)
    }
  }

  async function fetchPersonMedia(personId: string, useCache = true, snack = true) {
    if (personMediaPromises.has(personId)) {
      await personMediaPromises.get(personId)!
      return
    }
    if (useCache && personMedia.value.has(personId)) return

    try {
      const promise = peopleService.get(personId)
      personMediaPromises.set(personId, promise)
      const response = await promise
      personMediaPromises.delete(personId)
      personMedia.value.set(personId, response)
      triggerRef(personMedia)
    } catch (e) {
      personMediaPromises.delete(personId)
      if (snack) snackbarStore.error("Can't fetch person photos", e)
    }
  }

  async function updatePerson(personId: string, payload: UpdatePersonRequest) {
    try {
      await peopleService.update(personId, payload)
      const person = people.value.find((p) => p.id === personId)
      if (person && 'name' in payload) {
        person.name = payload.name ?? undefined
        triggerRef(people)
      }
      requestIdleCallback(() => {
        fetchPeople()
        fetchPersonMedia(personId, false)
      })
      return true
    } catch (e) {
      snackbarStore.error("Can't update person", e)
      return false
    }
  }

  async function mergePerson(personId: string, targetPersonId: string) {
    try {
      await peopleService.merge(personId, { targetPersonId })
      requestIdleCallback(() => {
        fetchPeople()
        fetchPersonMedia(personId, false, false)
      })
      return true
    } catch (e) {
      snackbarStore.error("Can't merge people", e)
      return false
    }
  }

  async function unmergePerson(personId: string) {
    const confirmed = await dialogs.confirm({
      title: 'Separate merged people?',
      description: 'This will split this person into separate groups.',
      confirmText: 'Separate',
      icon: 'mdi-account-multiple-remove',
      color: 'warning',
    })
    if (!confirmed) return false

    try {
      await peopleService.unmerge(personId)
      requestIdleCallback(() => {
        fetchPeople()
        fetchPersonMedia(personId, false)
      })
      await router.replace('/people')
      return true
    } catch (e) {
      snackbarStore.error("Can't unmerge person", e)
      return false
    }
  }

  function getPhotoThumb(person: PersonInfo, isDark: boolean) {
    const clusterId = person.faceThumbId ?? person.faceClusterIds[0]
    const url = peopleService.getFaceThumbnail(clusterId)
    if (url === '') {
      return isDark ? `/img/person-no-thumb-dark.png` : `/img/person-no-thumb.png`
    }
    return url
  }

  return {
    people,
    personMedia,

    fetchPeople,
    fetchPersonMedia,
    updatePerson,
    mergePerson,
    unmergePerson,
    getPhotoThumb,
  }
})
