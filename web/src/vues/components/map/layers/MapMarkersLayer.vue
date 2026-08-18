<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import {
  type GeoJSONFeature,
  GeoJSONSource,
  type Map as LibreMap,
  type MapSourceDataEvent,
  Marker,
} from 'maplibre-gl'
import type * as GeoJSON from 'geojson'
import type { MapPhotosResponse, SimpleTimelineItem } from '@/scripts/types/generated/timeline.ts'
import {
  createPhotosGeoJson,
  getFeatureCoordinates,
  getItemFromProperties,
  getThumbnailUrl,
} from '@/vues/components/map/layers/mapUtils.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { getVideoHeight } from '@/scripts/utils.ts'
import { useDebounceFn } from '@vueuse/core'
import { useRoute, useRouter } from 'vue-router'

export interface EmitMarkerSelected {
  key: string
  coords: [number, number]
}
export interface EmitClusterSelected {
  items: SimpleTimelineItem[]
  item: SimpleTimelineItem
}

const props = defineProps<{
  map: LibreMap
  mapPhotos: MapPhotosResponse | null
}>()

const emit = defineEmits<{
  'visible-items-changed': [items: SimpleTimelineItem[]]
  'marker-selected': [data: EmitMarkerSelected]
  'cluster-selected': [data: EmitClusterSelected]
}>()

const route = useRoute()
const router = useRouter()

let isUnmounted = false
let updateRun = 0
const markers: Record<string, Marker> = {}
const clusterPreviewCache = new Map<number, SimpleTimelineItem>()
let popupMarker: Marker | null = null

const currentVisibleIds = new Set<string>()
const visibleItems = ref<SimpleTimelineItem[]>([])
const selectedClusterItems = ref<SimpleTimelineItem[] | null>(null)
const selectedMarkerKey = ref<string | null>(null)
const selectedPopupItem = ref<SimpleTimelineItem | null>(null)
const selectedLngLat = ref<[number, number] | null>(null)

const photoItems = computed(() => {
  return props.mapPhotos?.items.map((p) => p.item).filter((p) => !!p) ?? []
})

function addMarkerLayers(loadedMap: LibreMap) {
  loadedMap.addLayer({
    id: 'cluster-helper',
    type: 'circle',
    source: 'photos',
    filter: ['has', 'point_count'],
    paint: {
      'circle-radius': 30,
      'circle-opacity': 0,
    },
  })

  loadedMap.addLayer({
    id: 'unclustered-point-helper',
    type: 'circle',
    source: 'photos',
    filter: ['!', ['has', 'point_count']],
    paint: {
      'circle-radius': 20,
      'circle-opacity': 0,
    },
  })
}

async function syncVisibleMarkers() {
  if (isUnmounted) return
  const loadedMap = props.map
  const run = ++updateRun
  const source = loadedMap.getSource('photos') as GeoJSONSource
  if (!source) return

  const clusterFeatures = loadedMap.queryRenderedFeatures({ layers: ['cluster-helper'] })
  const pointFeatures = loadedMap.queryRenderedFeatures({ layers: ['unclustered-point-helper'] })
  const newMarkers: Record<string, Marker> = {}
  const visibleItemMap = new Map<string, SimpleTimelineItem>()

  let clusterResults: Array<{
    clusterId: number
    count: number
    coords: [number, number]
    leaves: GeoJSON.Feature[]
  }> = []

  try {
    const promises = clusterFeatures.map(async (feature) => {
      const clusterId = Number(feature.properties.cluster_id)
      const count = Number(feature.properties.point_count)
      const coords = getFeatureCoordinates(feature)
      const leaves = await source.getClusterLeaves(clusterId, count, 0)
      return { clusterId, count, coords, leaves }
    })

    clusterResults = await Promise.all(promises)
  } catch {
    return // Source changed mid-query, abort execution
  }

  // Abort if unmounted or if a newer sync run has started
  if (isUnmounted || run !== updateRun) return

  // 1. Process clusters
  for (const res of clusterResults) {
    const { clusterId, count, coords, leaves } = res
    const previewItem = getClusterPreviewItem(clusterId, leaves)
    if (!previewItem) continue

    for (const leaf of leaves) {
      const item = getItemFromProperties(leaf.properties)
      if (item) visibleItemMap.set(item.id, item)
    }

    addOrUpdateClusterMarker(loadedMap, clusterId, previewItem, count, coords, newMarkers)
  }

  // 2. Process exact points
  for (const feature of pointFeatures) {
    const item = getItemFromProperties(feature.properties)
    if (!item) continue

    visibleItemMap.set(item.id, item)
    addOrUpdatePhotoMarker(loadedMap, item, getFeatureCoordinates(feature), newMarkers)
  }

  if (isUnmounted || run !== updateRun) return

  // Apply deep-diff cache matching for markers to save Vue computation overhead
  const newIds = new Set(visibleItemMap.keys())
  let hasChanged = newIds.size !== currentVisibleIds.size
  if (!hasChanged) {
    for (const id of newIds) {
      if (!currentVisibleIds.has(id)) {
        hasChanged = true
        break
      }
    }
  }

  if (hasChanged) {
    currentVisibleIds.clear()
    newIds.forEach((id) => currentVisibleIds.add(id))
    visibleItems.value = [...visibleItemMap.values()]
    emit('visible-items-changed', visibleItems.value)
  }

  removeHiddenMarkers(newMarkers)
  updateSelectedMarkerClasses()
}

function getClusterPreviewItem(clusterId: number, leaves: GeoJSON.Feature[]) {
  if (!clusterPreviewCache.has(clusterId)) {
    let firstProps = leaves[0]?.properties
    for (const leaf of leaves) {
      const props = leaf.properties
      if (!props) continue
      if (!firstProps || props.id > firstProps.id) {
        firstProps = props
      }
    }
    const previewItem = getItemFromProperties(firstProps ?? undefined)
    if (!previewItem) return null
    clusterPreviewCache.set(clusterId, previewItem)
  }
  return clusterPreviewCache.get(clusterId)!
}

// --- Marker DOM Creation & Lifecycle ---
function addOrUpdateClusterMarker(
  loadedMap: LibreMap,
  clusterId: number,
  item: SimpleTimelineItem,
  count: number,
  coords: [number, number],
  visibleMarkers: Record<string, Marker>,
) {
  const key = `cluster-${clusterId}`
  return addOrUpdateMarker(
    loadedMap,
    key,
    coords,
    visibleMarkers,
    () => createClusterMarkerElement(item, count),
    (el) => updateClusterMarkerElement(el, count),
  )
}

function addOrUpdatePhotoMarker(
  loadedMap: LibreMap,
  item: SimpleTimelineItem,
  coords: [number, number],
  visibleMarkers: Record<string, Marker>,
) {
  const key = `photo-${item.id}`
  if (visibleMarkers[key]) return visibleMarkers[key]

  return addOrUpdateMarker(loadedMap, key, coords, visibleMarkers, () =>
    createPhotoMarkerElement(item),
  )
}

function addOrUpdateMarker(
  loadedMap: LibreMap,
  key: string,
  coords: [number, number],
  visibleMarkers: Record<string, Marker>,
  createElement: () => HTMLElement,
  updateElement?: (el: HTMLElement) => void,
) {
  if (isUnmounted) return

  let marker = markers[key]
  if (!marker) {
    const element = createElement()
    element.addEventListener('click', (e) => {
      e.preventDefault()
      e.stopPropagation()
      const lngLat = markers[key]?.getLngLat()
      handleMarkerClick(key, lngLat ? [lngLat.lng, lngLat.lat] : coords)
    })
    marker = markers[key] = new Marker({
      element,
      anchor: 'center',
    }).setLngLat(coords)
  } else {
    marker.setLngLat(coords)
    updateElement?.(marker.getElement())
  }

  visibleMarkers[key] = marker
  if (!marker.getElement().parentElement) marker.addTo(loadedMap)
  return marker
}

function removeHiddenMarkers(visibleMarkers: Record<string, Marker>) {
  for (const key in markers) {
    if (!visibleMarkers[key]) {
      markers[key].remove()
      delete markers[key]
    }
  }
}

function removeAllMarkers() {
  Object.values(markers).forEach((m) => m.remove())
  for (const key in markers) {
    delete markers[key]
  }
}

const createPhotoMarkerElement = (item: SimpleTimelineItem) => {
  const el = document.createElement('div')
  const imageArea = 2500
  const markerWidth = Math.sqrt(imageArea * item.ratio)
  const markerHeight = Math.sqrt(imageArea * (1 / item.ratio))
  el.className = 'map-photo-marker'
  if (item.isVideo) el.classList.add('map-photo-marker-video')
  el.style.width = `${Math.round(markerWidth)}px`
  el.style.height = `${markerHeight}px`
  el.style.backgroundImage = `url(${getThumbnailUrl(item, markerHeight)})`
  return el
}

const createClusterMarkerElement = (item: SimpleTimelineItem, count: number) => {
  const el = document.createElement('div')
  const visual = document.createElement('div')
  const badge = document.createElement('span')
  el.className = 'map-cluster-marker'
  visual.className = 'map-cluster-visual'
  badge.className = 'map-cluster-count'
  visual.style.backgroundImage = `url(${getThumbnailUrl(item, 52)})`
  visual.append(badge)
  el.append(visual)
  updateClusterMarkerElement(el, count)
  return el
}

const updateClusterMarkerElement = (el: HTMLElement, count: number) => {
  const badge = el.querySelector<HTMLElement>('.map-cluster-count')
  if (badge) badge.textContent = String(count)
}

// --- Marker Interactions & Popups ---
async function handleMarkerClick(key: string, coords: [number, number]) {
  if (selectedMarkerKey.value === key) {
    clearMarkerSelection()
    return
  }

  selectedMarkerKey.value = key
  selectedLngLat.value = coords
  updateSelectedMarkerClasses()

  if (key.startsWith('cluster-')) {
    const clusterId = Number(key.replace('cluster-', ''))
    await selectCluster(clusterId)
  } else {
    const item = photoItems.value.find((photoItem) => `photo-${photoItem.id}` === key)
    if (!item) return
    selectedClusterItems.value = null
    selectedPopupItem.value = item
    showPopup(item, coords)
    emit('marker-selected', { key, coords })
  }
}

async function selectCluster(clusterId: number) {
  const map = props.map
  const source = map.getSource('photos') as GeoJSONSource
  const clusterFeature = map
    .queryRenderedFeatures({ layers: ['cluster-helper'] })
    .find((feature) => Number(feature.properties.cluster_id) === clusterId)
  const count = Number(clusterFeature?.properties.point_count)

  try {
    const leaves = await source.getClusterLeaves(clusterId, Number.isFinite(count) ? count : 100, 0)
    const items: SimpleTimelineItem[] = []
    for (const leaf of leaves) {
      const item = getItemFromProperties(leaf.properties)
      if (item) items.push(item)
    }
    selectedClusterItems.value = items
    selectedPopupItem.value = getClusterPreviewItem(clusterId, leaves)
    if (selectedPopupItem.value && selectedLngLat.value) {
      showPopup(selectedPopupItem.value, selectedLngLat.value)
    }
    emit('cluster-selected', {
      items,
      item: selectedPopupItem.value!,
    })
  } catch (err) {
    console.warn('Failed to retrieve cluster leaves:', err)
  }
}

function clearMarkerSelection() {
  selectedClusterItems.value = null
  selectedMarkerKey.value = null
  selectedPopupItem.value = null
  selectedLngLat.value = null
  updateSelectedMarkerClasses()
  closePopup()
}

function updateSelectedMarkerClasses() {
  for (const [key, marker] of Object.entries(markers)) {
    marker.getElement().classList.toggle('map-marker-selected', key === selectedMarkerKey.value)
  }
}

function showPopup(item: SimpleTimelineItem, coords: [number, number]) {
  closePopup()
  const map = props.map

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
    clearMarkerSelection()
  })

  popupEl.addEventListener('click', (e) => {
    e.stopPropagation()
    router.push({ path: `/map/view/${item.id}`, query: route.query })
  })

  popupEl.append(mediaEl, closeButton)

  popupMarker = new Marker({
    element: popupEl,
    anchor: 'bottom',
    offset: [0, -38],
  })
    .setLngLat(coords)
    .addTo(map)
}

function closePopup() {
  popupMarker?.remove()
  popupMarker = null
}

const debouncedUpdate = useDebounceFn(syncVisibleMarkers, 80)

function handleSourceData(e: MapSourceDataEvent) {
  if (e.sourceId === 'photos' && e.isSourceLoaded) debouncedUpdate()
}

function handleMapClick() {
  clearMarkerSelection()
}

function setupResources() {
  cleanupResources()

  if (props.mapPhotos) {
    props.map.addSource('photos', {
      type: 'geojson',
      data: createPhotosGeoJson(props.mapPhotos),
      cluster: true,
      clusterMaxZoom: 17,
      clusterRadius: 48,
    })
  }

  addMarkerLayers(props.map)
  props.map.once('idle', () => {
    if (!isUnmounted) {
      syncVisibleMarkers()
    }
  })
}

function cleanupResources() {
  clearMarkerSelection()
  removeAllMarkers()
  currentVisibleIds.clear()
  visibleItems.value = []

  const layersToRemove = ['cluster-helper', 'unclustered-point-helper']
  layersToRemove.forEach((layerId) => {
    if (props.map.getLayer(layerId)) props.map.removeLayer(layerId)
  })

  if (props.map.getSource('photos')) {
    props.map.removeSource('photos')
  }
}

function handleStyleLoad() {
  setupResources()
}

onMounted(() => {
  isUnmounted = false
  setupResources()
  props.map.on('zoomend', debouncedUpdate)
  props.map.on('moveend', debouncedUpdate)
  props.map.on('sourcedata', handleSourceData)
  props.map.on('click', handleMapClick)
  props.map.on('style.load', handleStyleLoad)
})

onUnmounted(() => {
  isUnmounted = true
  updateRun++
  debouncedUpdate.cancel?.()

  props.map.off('zoomend', debouncedUpdate)
  props.map.off('moveend', debouncedUpdate)
  props.map.off('sourcedata', handleSourceData)
  props.map.off('click', handleMapClick)
  props.map.off('style.load', handleStyleLoad)
  cleanupResources()
  clusterPreviewCache.clear()
})

watch(
  () => props.mapPhotos,
  (newPhotos) => {
    if (!props.map || isUnmounted) return
    const source = props.map.getSource('photos') as GeoJSONSource | undefined
    if (source && newPhotos) {
      source.setData(createPhotosGeoJson(newPhotos))
      props.map.triggerRepaint()
      setTimeout(() => {
        if (!isUnmounted) {
          debouncedUpdate()
        }
      }, 75)
    } else if (newPhotos) {
      setupResources()
    }
  },
  { deep: true },
)
</script>

<template>
  <slot />
</template>

<style>
/* --- Custom Map Markers & Cluster Visuals --- */
.map-photo-marker,
.map-cluster-visual {
  background-color: rgba(20, 20, 24, 0.65);
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  border: 2px solid white;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  cursor: pointer;
  z-index: 1;
}

.map-photo-marker {
  border-radius: 8px;
  overflow: hidden;
}

.map-photo-marker-video {
  border-color: #ff9800;
}

.map-cluster-marker {
  width: 52px;
  height: 52px;
  cursor: pointer;
  overflow: visible;
  z-index: 1;
}

.map-cluster-visual {
  width: 100%;
  height: 100%;
  position: relative;
  border-radius: 50%;
  overflow: visible;
}

.map-cluster-count {
  position: absolute;
  right: -7px;
  top: -7px;
  min-width: 22px;
  height: 22px;
  padding: 0 5px;
  border-radius: 999px;
  border: 2px solid white;
  box-sizing: border-box;
  background: rgb(var(--v-theme-primary));
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  text-align: center;
  box-shadow: 0 2px 7px rgba(0, 0, 0, 0.3);
  pointer-events: none;
  z-index: 1;
}

.map-photo-marker:hover,
.map-cluster-marker:hover .map-cluster-visual {
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.35),
    0 0 0 3px rgba(255, 255, 255, 0.35);
  z-index: 10;
}

.map-photo-marker.map-marker-selected,
.map-cluster-marker.map-marker-selected .map-cluster-visual {
  border-color: rgb(var(--v-theme-secondary));
  box-shadow:
    0 2px 8px rgba(0, 0, 0, 0.35),
    0 0 0 4px rgba(var(--v-theme-secondary), 0.45);
  z-index: 2;
}

/* --- Map Media Popup --- */
.map-media-popup {
  position: relative;
  border: 2px solid rgba(255, 255, 255, 0.86);
  border-radius: 12px;
  background: rgba(20, 20, 24, 0.78);
  box-shadow: 0 12px 28px rgba(0, 0, 0, 0.35);
  cursor: pointer;
  z-index: 20;
}

.map-media-popup::after {
  content: '';
  position: absolute;
  left: 50%;
  bottom: -14px;
  width: 0;
  height: 0;
  border-left: 13px solid transparent;
  border-right: 13px solid transparent;
  border-top: 14px solid rgba(255, 255, 255, 0.86);
  transform: translateX(-50%);
}

.map-media-popup::before {
  content: '';
  position: absolute;
  left: 50%;
  bottom: -11px;
  width: 0;
  height: 0;
  border-left: 10px solid transparent;
  border-right: 10px solid transparent;
  border-top: 11px solid rgba(20, 20, 24, 0.78);
  transform: translateX(-50%);
  z-index: 1;
}

.map-media-popup-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 10px;
}

.map-media-popup-close {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.45);
  color: white;
  cursor: pointer;
  font-size: 18px;
  display: grid;
  place-items: center;
  z-index: 1;
}

.map-media-popup-close:hover {
  background: rgba(0, 0, 0, 0.68);
}
</style>
