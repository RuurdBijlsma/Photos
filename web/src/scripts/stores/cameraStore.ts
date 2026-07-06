import { defineStore } from 'pinia'
import { shallowRef, triggerRef } from 'vue'
import type { CameraInfo, FullCameraPhotosResponse } from '@/scripts/types/generated/timeline.ts'
import { useSnackbarsStore } from '@/scripts/stores/snackbarStore.ts'
import { useObjStorage } from '@/scripts/utils.ts'
import cameraService from '@/scripts/services/cameraService.ts'

export const useCameraStore = defineStore('cameras', () => {
  const snackbarStore = useSnackbarsStore()

  const cameras = useObjStorage<CameraInfo[]>('userCameras', [])
  const cameraMedia = shallowRef(new Map<string, FullCameraPhotosResponse>())
  const cameraMediaPromises = new Map<string, Promise<FullCameraPhotosResponse>>()

  async function fetchCameras() {
    try {
      const response = await cameraService.list()
      console.log('Cameras list', response.cameras)
      cameras.value = response.cameras
    } catch (e) {
      snackbarStore.error("Can't fetch cameras", e)
    }
  }

  async function fetchCameraMedia(
    cameraMake: string,
    cameraModel: string,
    useCache = true,
    snack = true,
  ) {
    const cameraId = cameraMake + cameraModel
    if (cameraMediaPromises.has(cameraId)) {
      await cameraMediaPromises.get(cameraId)!
      return
    }
    if (useCache && cameraMedia.value.has(cameraId)) return

    try {
      const promise = cameraService.get(cameraMake, cameraModel)
      cameraMediaPromises.set(cameraId, promise)
      const response = await promise
      cameraMedia.value.set(cameraId, response)
      triggerRef(cameraMedia)
    } catch (e) {
      if (snack) snackbarStore.error("Can't fetch person photos", e)
    } finally {
      cameraMediaPromises.delete(cameraId)
    }
  }

  return {
    cameras,
    cameraMedia,
    fetchCameraMedia,
    fetchCameras,
  }
})
