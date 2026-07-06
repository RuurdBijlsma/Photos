<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import maplibregl, { type Map as LibreMap, type MapOptions } from 'maplibre-gl'
import BaseMap, { type StyleName } from '@/vues/components/map/BaseMap.vue'
import MapLayerSelector from '@/vues/components/map/MapLayerSelector.vue'
import MapDateFilter from '@/vues/components/map/MapDateFilter.vue'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import { getThumbnailHeight, getVideoHeight } from '@/scripts/utils.ts'
import { useDebounceFn, useStorage } from '@vueuse/core'
import type {
  MapPhotoItem,
  MapPhotosResponse,
  SimpleTimelineItem,
} from '@/scripts/types/generated/timeline.ts'
import { useRouter, useRoute } from 'vue-router'

// Tweakable visual configuration for the map representation
const HEATMAP_CONFIG = {
  // The zoom level where point circles begin to show up
  pointMinZoom: 13,
  heatmapMaxZoom: 16,

  // Heatmap Intensity: global density multiplier by zoom.
  // Kept low when zoomed out to show structure, ramped up as points separate.
  intensity: [
    [0, 0.2], // World view – gentle presence
    [5, 0.35], // Continent scale
    [10, 0.7], // Regional clusters become distinct
    [14, 1.0], // Local peaks stand out clearly
  ],

  // Heatmap Radius: blending radius in pixels per zoom.
  // A smooth decay that prevents oceans from flooding while keeping cities connected.
  radius: [
    [0, 12], // Tight at global level to avoid ocean smearing
    [5, 18], // Merges distant points into corridors
    [10, 30], // Natural separation of neighbourhoods
    [16, 25], // Crisp individual hotspots before fading
  ],

  // Heatmap Opacity: seamless crossover from heatmap to point markers.
  opacity: [
    [12, 0.75],
    [16, 0],
  ],

  // Color Stops: classic thermal gradient (transparent → cold → hot → peak white).
  // Uses solid colours so that layer blending is controlled only by heatmap-opacity.
  colorStops: [
    [0, 'rgba(0, 0, 0, 0)'], // fully invisible boundary
    [0.05, 'rgb(76 70 184 / 0.7)'], // deep indigo for lowest density
    [0.2, 'rgba(0, 140, 200, 0.7)'], // rich cyan/blue
    [0.4, 'rgba(40, 200, 100, 0.7)'], // vivid green
    [0.6, 'rgba(240, 220, 40, 0.7)'], // warm yellow
    [0.8, 'rgb(214 116 49 / 0.7)'], // intense orange
    [1, 'rgb(213 75 75 / 0.7)'], // bright near-white for extreme peaks
  ],

  // Point markers – subtle circles at high zoom to show exact location.
  point: {
    color: 'rgb(80, 30, 120)', // deep purple, visible on most maps
    strokeColor: '#ffffff',
    strokeWidth: 1.8,
    radius: [
      [13, 4],
      [17, 11],
    ],
    opacity: [
      [13, 0],
      [15.5, 0.8],
      [16, 1],
    ],
  },
}

const MAP_STYLES = [
  { key: 'LIBERTY', label: 'Light Map', thumb: 'img/map-thumb/LIBERTY.png' },
  { key: 'SATELLITE', label: 'Satellite', thumb: 'img/map-thumb/SATELLITE.png' },
  { key: 'TERRAIN', label: 'Terrain', thumb: 'img/map-thumb/TERRAIN.png' },
  { key: 'WATERCOLOR', label: 'Watercolor', thumb: 'img/map-thumb/WATERCOLOR.png' },
  { key: 'DARK_COLORFUL', label: 'Dark Map', thumb: 'img/map-thumb/DARK_COLORFUL.png' },
] as const

const DEFAULT_MAP_OPTIONS = {
  center: { lat: 40, lng: 0 },
  zoom: 3,
  attributionControl: { compact: true },
}

interface Props {
  mapPhotos: MapPhotosResponse | null
  loadCoord: { lat: number; lng: number } | null
}

export interface DateFilter {
  startDate: Date | null
  endDate: Date | null
  active: boolean
  startGranularity: 'month' | 'day'
  endGranularity: 'month' | 'day'
}

const props = defineProps<Props>()
export interface EmitMarkerSelected {
  key: string
  coords: [number, number]
}
export interface EmitClusterSelected {
  items: SimpleTimelineItem[]
  item: SimpleTimelineItem
}

const emit = defineEmits<{
  'visible-items-changed': [items: SimpleTimelineItem[]]
  'marker-selected': [data: EmitMarkerSelected]
  'cluster-selected': [data: EmitClusterSelected]
  'date-filter-change': [payload: { isDragging: boolean; dateFilter: DateFilter }]
}>()

const route = useRoute()
const router = useRouter()

// --- State & Storage ---
const mapMode = useStorage<'markers' | 'heatmap'>('mapLayerMode', 'heatmap')
let map: null | maplibregl.Map = null
let initialized = false
let updateRun = 0

const photoIdToOrder = new Map<string, number>()
const markers: Record<string, maplibregl.Marker> = {}
const clusterPreviewCache = new Map<number, SimpleTimelineItem>()
let popupMarker: maplibregl.Marker | null = null

// Keep track of active viewport IDs to prevent redundant state updates and re-renders
const currentVisibleIds = new Set<string>()
const visibleItems = ref<SimpleTimelineItem[]>([])
const selectedClusterItems = ref<SimpleTimelineItem[] | null>(null)
const selectedMarkerKey = ref<string | null>(null)
const selectedPopupItem = ref<SimpleTimelineItem | null>(null)
const selectedLngLat = ref<[number, number] | null>(null)

const dateFilter = ref<DateFilter>({
  startDate: null,
  endDate: null,
  active: false,
  startGranularity: 'month',
  endGranularity: 'month',
})

const mapOptions = ref<Omit<MapOptions, 'container' | 'style'> | null>(null)
const currentStyle = useStorage<StyleName>('mapCurrentStyle', 'LIBERTY')

// --- Computed Properties ---
const nextStyle = computed(() => {
  const currentIndex = MAP_STYLES.findIndex((s) => s.key === currentStyle.value)
  const nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % MAP_STYLES.length
  return MAP_STYLES[nextIndex]
})

function cycleStyle() {
  currentStyle.value = nextStyle.value.key
}

const photoItems = computed(() => {
  return props.mapPhotos?.items.map((p) => p.item).filter((p) => !!p) ?? []
})

// --- Fetch Data & Handlers ---
function handleDateFilterChange(payload: { isDragging: boolean }) {
  emit('date-filter-change', {
    isDragging: payload.isDragging,
    dateFilter: dateFilter.value,
  })
}

// --- Map Lifecycle & Source Configuration ---
function handleMapLoad(loadedMap: LibreMap) {
  map = loadedMap
  initializeMap()
}

function handleStyleLoad(loadedMap: LibreMap | undefined) {
  if (props.mapPhotos === null || !loadedMap) return
  rebuildMapResources(loadedMap)
}

function initializeMap() {
  if (map === null || props.mapPhotos === null || initialized) return
  initialized = true

  map.addControl(
    new maplibregl.NavigationControl({
      showCompass: true,
      showZoom: true,
      visualizePitch: true,
    }),
    'bottom-right',
  )

  rebuildMapResources(map)

  const updateViewportData = () => {
    if (!map) return
    if (mapMode.value === 'markers') {
      syncVisibleMarkers(map)
    } else {
      syncVisibleHeatmapItems(map)
    }
  }

  const debouncedUpdate = useDebounceFn(updateViewportData, 80)

  map.on('zoomend', debouncedUpdate)
  map.on('moveend', debouncedUpdate)
  map.on('data', (e: maplibregl.MapDataEvent & { sourceId?: string; isSourceLoaded?: boolean }) => {
    if (e.sourceId === 'photos' && e.isSourceLoaded) debouncedUpdate()
  })
  map.on('click', () => {
    if (mapMode.value === 'markers') clearMarkerSelection()
  })

  updateViewportData()
}

/**
 * Rebuilds map sources and layers from scratch based on the visualization mode
 */
function rebuildMapResources(loadedMap: LibreMap) {
  // 1. Clear active selections and UI helpers
  clearMarkerSelection()
  removeAllMarkers()
  currentVisibleIds.clear()
  visibleItems.value = []

  // 2. Clear pre-existing layers safely
  const layersToRemove = [
    'photos-heat',
    'photos-point',
    'photos-helper',
    'cluster-helper',
    'unclustered-point-helper',
  ]
  layersToRemove.forEach((layerId) => {
    if (loadedMap.getLayer(layerId)) loadedMap.removeLayer(layerId)
  })

  // 3. Rebuild sources (dynamic configuration for clusters vs heatmap)
  if (loadedMap.getSource('photos')) {
    loadedMap.removeSource('photos')
  }

  if (props.mapPhotos) {
    loadedMap.addSource('photos', {
      type: 'geojson',
      data: createPhotosGeoJson(props.mapPhotos),
      ...(mapMode.value === 'markers'
        ? {
            cluster: true,
            clusterMaxZoom: 17,
            clusterRadius: 48,
          }
        : {}),
    })
  }

  // 4. Attach appropriate configuration layers and schedule initial sync
  if (mapMode.value === 'markers') {
    addMarkerLayers(loadedMap)
    loadedMap.once('idle', () => {
      syncVisibleMarkers(loadedMap)
    })
  } else {
    addHeatmapLayers(loadedMap)
    loadedMap.once('idle', () => {
      syncVisibleHeatmapItems(loadedMap)
    })
  }
}

function createPhotosGeoJson(photos: MapPhotosResponse): GeoJSON.FeatureCollection<GeoJSON.Point> {
  return {
    type: 'FeatureCollection',
    features: photos.items.map((p) => ({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: getLngLat(p),
      },
      properties: {
        id: p.item?.id,
        hasThumbnails: p.item?.hasThumbnails,
        isVideo: p.item?.isVideo,
        durationMs: p.item?.durationMs,
        ratio: p.item?.ratio,
      },
    })),
  }
}

// --- Heatmap Setup & Syncer ---
function addHeatmapLayers(loadedMap: LibreMap) {
  // Heatmap Layer using configured intensity, color bands, and radius curves
  loadedMap.addLayer({
    id: 'photos-heat',
    type: 'heatmap',
    source: 'photos',
    maxzoom: HEATMAP_CONFIG.heatmapMaxZoom,
    paint: {
      'heatmap-intensity': [
        'interpolate',
        ['linear'],
        ['zoom'],
        ...HEATMAP_CONFIG.intensity.flat(),
      ],
      'heatmap-color': [
        'interpolate',
        ['linear'],
        ['heatmap-density'],
        ...HEATMAP_CONFIG.colorStops.flat(),
      ],
      'heatmap-radius': ['interpolate', ['linear'], ['zoom'], ...HEATMAP_CONFIG.radius.flat()],
      'heatmap-opacity': ['interpolate', ['linear'], ['zoom'], ...HEATMAP_CONFIG.opacity.flat()],
    },
  })

  // Exact point marker circles mapping to the zoom configuration
  loadedMap.addLayer({
    id: 'photos-point',
    type: 'circle',
    source: 'photos',
    minzoom: HEATMAP_CONFIG.pointMinZoom,
    paint: {
      'circle-radius': ['interpolate', ['linear'], ['zoom'], ...HEATMAP_CONFIG.point.radius.flat()],
      'circle-color': HEATMAP_CONFIG.point.color,
      'circle-stroke-color': HEATMAP_CONFIG.point.strokeColor,
      'circle-stroke-width': HEATMAP_CONFIG.point.strokeWidth,
      'circle-opacity': [
        'interpolate',
        ['linear'],
        ['zoom'],
        ...HEATMAP_CONFIG.point.opacity.flat(),
      ],
    },
  })

  // Invisible layer used solely to query visible items in the viewport
  loadedMap.addLayer({
    id: 'photos-helper',
    type: 'circle',
    source: 'photos',
    paint: {
      'circle-radius': 10,
      'circle-opacity': 0,
    },
  })
}

function syncVisibleHeatmapItems(loadedMap: LibreMap) {
  if (!loadedMap.getLayer('photos-helper')) return

  const helperFeatures = loadedMap.queryRenderedFeatures({ layers: ['photos-helper'] })
  if (helperFeatures.length === 0 && visibleItems.value.length === 0) {
    return
  }

  const newItems: SimpleTimelineItem[] = []
  const newIds = new Set<string>()

  for (const feature of helperFeatures) {
    const id = feature.properties?.id
    if (id && !newIds.has(id)) {
      const item = getItemFromProperties(feature.properties)
      if (item) {
        newIds.add(id)
        newItems.push(item)
      }
    }
  }

  // Skip updates if the on-screen set is identical
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
    visibleItems.value = newItems
    emit('visible-items-changed', newItems)
  }
}

// --- Markers & Clusters Setup & Syncer ---
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

async function syncVisibleMarkers(loadedMap: LibreMap) {
  const run = ++updateRun
  const source = loadedMap.getSource('photos') as maplibregl.GeoJSONSource
  if (!source) return

  const clusterFeatures = loadedMap.queryRenderedFeatures({ layers: ['cluster-helper'] })
  const pointFeatures = loadedMap.queryRenderedFeatures({ layers: ['unclustered-point-helper'] })
  const newMarkers: Record<string, maplibregl.Marker> = {}
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

  if (run !== updateRun) return

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
  visibleMarkers: Record<string, maplibregl.Marker>,
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
  visibleMarkers: Record<string, maplibregl.Marker>,
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
  visibleMarkers: Record<string, maplibregl.Marker>,
  createElement: () => HTMLElement,
  updateElement?: (el: HTMLElement) => void,
) {
  let marker = markers[key]
  if (!marker) {
    const element = createElement()
    element.addEventListener('click', (e) => {
      e.preventDefault()
      e.stopPropagation()
      const lngLat = markers[key]?.getLngLat()
      handleMarkerClick(key, lngLat ? [lngLat.lng, lngLat.lat] : coords)
    })
    marker = markers[key] = new maplibregl.Marker({
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

function removeHiddenMarkers(visibleMarkers: Record<string, maplibregl.Marker>) {
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
  }
}

async function selectCluster(clusterId: number) {
  if (!map) return
  const source = map.getSource('photos') as maplibregl.GeoJSONSource
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
  if (!map) return

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

  popupMarker = new maplibregl.Marker({
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

// --- Utilities ---
function getLngLat(item: MapPhotoItem): [number, number] {
  return [item.longitude, item.latitude]
}

function getFeatureCoordinates(feature: maplibregl.MapGeoJSONFeature): [number, number] {
  return (feature.geometry as GeoJSON.Point).coordinates as [number, number]
}

const getItemFromProperties = (
  properties: maplibregl.GeoJSONFeature['properties'] | null | undefined,
): SimpleTimelineItem | undefined => {
  if (!properties?.id) return undefined
  const ratio = Number(properties.ratio)
  const durationMs = Number(properties.durationMs)
  return {
    id: String(properties.id),
    isVideo: Boolean(properties.isVideo),
    hasThumbnails: Boolean(properties.hasThumbnails),
    ...(Number.isFinite(durationMs) && durationMs > 0 ? { durationMs } : {}),
    ratio: Number.isFinite(ratio) && ratio > 0 ? ratio : 1,
  }
}

const getThumbnailUrl = (item: SimpleTimelineItem, markerHeight: number) => {
  return mediaItemService.getPhotoThumbnail(
    item.id,
    getThumbnailHeight(markerHeight),
    !item.hasThumbnails,
  )
}

function zoomToFitAll() {
  if (!map || !props.mapPhotos) return
  const locations = props.mapPhotos.items
  if (locations.length === 0) return

  if (locations.length === 1) {
    const [location] = locations
    map.flyTo({
      center: getLngLat(location!),
      zoom: 11,
    })
    return
  }

  const bounds = locations.reduce(
    (photoBounds, item) => {
      return photoBounds.extend([item.longitude, item.latitude])
    },
    new maplibregl.LngLatBounds(getLngLat(locations[0]!), getLngLat(locations[0]!)),
  )

  map.fitBounds(bounds, {
    padding: 80,
    maxZoom: 14,
    duration: 1200,
  })
}

function getInitialMapOptions(photos: MapPhotosResponse): Omit<MapOptions, 'container' | 'style'> {
  if (props.loadCoord) {
    return {
      ...DEFAULT_MAP_OPTIONS,
      center: props.loadCoord,
      zoom: 16,
    }
  }
  const locations = photos.items
  if (locations.length === 0) return DEFAULT_MAP_OPTIONS

  if (locations.length === 1) {
    const [location] = locations
    return {
      ...DEFAULT_MAP_OPTIONS,
      center: getLngLat(location),
      zoom: 11,
    }
  }

  const bounds = locations.reduce(
    (photoBounds, item) => {
      return photoBounds.extend([item.longitude, item.latitude])
    },
    new maplibregl.LngLatBounds(getLngLat(locations[0]), getLngLat(locations[0])),
  )

  return {
    ...DEFAULT_MAP_OPTIONS,
    bounds,
    fitBoundsOptions: {
      padding: 80,
      maxZoom: 14,
    },
  }
}

// --- Initialization ---
if (props.mapPhotos) {
  props.mapPhotos.items.forEach((p, index) => {
    if (p.item?.id) {
      photoIdToOrder.set(p.item.id, index)
    }
  })
  mapOptions.value = getInitialMapOptions(props.mapPhotos)
}

// --- Watchers ---
watch(
  () => props.mapPhotos,
  (newPhotos) => {
    if (newPhotos) {
      newPhotos.items.forEach((p, index) => {
        if (p.item?.id) {
          photoIdToOrder.set(p.item.id, index)
        }
      })
      if (!mapOptions.value) {
        mapOptions.value = getInitialMapOptions(newPhotos)
      }
    }

    if (map && newPhotos) {
      const source = map.getSource('photos') as maplibregl.GeoJSONSource | undefined
      if (source) {
        source.setData(createPhotosGeoJson(newPhotos))
        map.triggerRepaint()
        setTimeout(() => {
          if (map) {
            if (mapMode.value === 'markers') {
              syncVisibleMarkers(map)
            } else {
              syncVisibleHeatmapItems(map)
            }
          }
        }, 75)
      }
    }
  },
  { deep: true },
)

watch(mapMode, () => {
  if (map && initialized) {
    rebuildMapResources(map)
  }
})

watch(
  () => props.loadCoord,
  (newVal, oldVal) => {
    if (!map || !props.loadCoord) return
    if (JSON.stringify(newVal) === JSON.stringify(oldVal)) return
    map.flyTo({
      center: props.loadCoord,
      zoom: 17,
    })
  },
)

onUnmounted(() => {
  removeAllMarkers()
  closePopup()
  clusterPreviewCache.clear()
})

defineExpose({
  zoomToFitAll,
})
</script>

<template>
  <div class="map-container">
    <v-theme-provider with-background class="map-wrapper" theme="light">
      <base-map
        v-if="mapOptions"
        :map-style="currentStyle"
        class="map-instance"
        :map-options="mapOptions"
        @load="handleMapLoad"
        @style-load="handleStyleLoad"
      />
      <div v-else class="map-loading">
        <v-progress-circular indeterminate color="primary" />
      </div>

      <!-- Date Range Filter -->
      <map-date-filter
        v-if="props.mapPhotos"
        :theme="currentStyle === 'DARK_COLORFUL' ? 'dark' : 'light'"
        :model-value="dateFilter"
        @change="handleDateFilterChange"
        @update:model-value="dateFilter = $event"
      />

      <!-- Map Style / Layer Selector -->
      <map-layer-selector
        v-if="mapOptions"
        :current-style="currentStyle"
        :map-mode="mapMode"
        :next-style="nextStyle"
        :map-styles="MAP_STYLES"
        @update:current-style="currentStyle = $event"
        @update:map-mode="mapMode = $event"
        @cycle-style="cycleStyle"
      />
    </v-theme-provider>
  </div>
</template>

<style scoped>
.map-container {
  width: 100%;
  height: 100%;
}

/* --- Map Wrapper & Controls --- */
.map-wrapper {
  width: 100%;
  height: 100%;
  position: relative;
}

.map-instance,
.map-instance > div {
  width: 100%;
  height: 100%;
}

.map-loading {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
}
</style>

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
