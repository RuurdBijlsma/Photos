import { Marker, type Map as LibreMap } from 'maplibre-gl'
import type { LocationQueryRaw, Router } from 'vue-router'
import type { SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { getThumbnailUrl } from '@/scripts/mapUtils/mapUtils.ts'
import { getVideoHeight } from '@/scripts/utils.ts'

export interface MapMediaPopupOptions {
  map: LibreMap
  item: SimpleTimelineItem
  coords: [number, number]
  router: Router
  query?: LocationQueryRaw
  offset?: [number, number]
  onClose?: () => void
}

export class MapMediaPopupController {
  private popupMarker: Marker | null = null

  show(options: MapMediaPopupOptions) {
    this.close()

    const { map, item, coords, router, query = {}, offset = [0, -38], onClose } = options

    const popupArea = 300 ** 2
    const popupWidth = Math.sqrt(popupArea * item.ratio)
    const popupHeight = Math.sqrt(popupArea * (1 / item.ratio))

    const popupEl = document.createElement('div')
    popupEl.style.width = `${popupWidth}px`
    popupEl.style.height = `${popupHeight}px`
    popupEl.className = 'map-media-popup'

    const closeButton = document.createElement('button')
    let mediaEl: HTMLImageElement | HTMLVideoElement

    if (item.isVideo) {
      const videoEl = document.createElement('video')
      videoEl.autoplay = true
      videoEl.muted = true
      videoEl.loop = true
      videoEl.playsInline = true
      videoEl.poster = getThumbnailUrl(item, 480)
      videoEl.src = mediaItemService.getVideo(item.id, getVideoHeight(480), !item.hasThumbnails)
      mediaEl = videoEl
    } else {
      const imageEl = document.createElement('img')
      imageEl.src = getThumbnailUrl(item, 480)
      imageEl.alt = ''
      mediaEl = imageEl
    }

    mediaEl.className = 'map-media-popup-image'
    closeButton.className = 'map-media-popup-close'
    closeButton.type = 'button'
    closeButton.textContent = '×'

    closeButton.addEventListener('click', (e) => {
      e.preventDefault()
      e.stopPropagation()
      this.close()
      onClose?.()
    })

    popupEl.addEventListener('click', (e) => {
      e.stopPropagation()
      router.push({ path: `/map/view/${item.id}`, query })
    })

    popupEl.append(mediaEl, closeButton)

    this.popupMarker = new Marker({
      element: popupEl,
      anchor: 'bottom',
      offset,
    })
      .setLngLat(coords)
      .addTo(map)
  }

  close() {
    if (this.popupMarker) {
      this.popupMarker.remove()
      this.popupMarker = null
    }
  }
}
