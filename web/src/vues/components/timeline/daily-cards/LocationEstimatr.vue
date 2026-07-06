<script setup lang="ts">
import { ref, computed, watch, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import type { CollectionMediaItem, DailyCardResponse } from '@/scripts/types/api/dailyCards.ts'
import { useObjStorage } from '@/scripts/utils.ts'
import mediaItemService from '@/scripts/services/mediaItemService.ts'
import BaseMap from '@/vues/components/map/BaseMap.vue'
import type { StyleName } from '@/vues/components/map/BaseMap.vue'
import maplibregl from 'maplibre-gl'
import { useDailyCardStore } from '@/scripts/stores/timeline/dailyCardStore.ts'
import MediaViewer from '@/vues/components/viewer/MediaViewer.vue'
import { useMediaItemStore } from '@/scripts/stores/timeline/mediaItemStore.ts'

const props = defineProps<{
  card: DailyCardResponse
}>()

const router = useRouter()
const cardStore = useDailyCardStore()
const mediaItemStore = useMediaItemStore()

export interface EstimatrRound {
  latitude: number
  longitude: number
  mediaItem: CollectionMediaItem
}

export interface EstimatrPayload {
  rounds: EstimatrRound[]
  areaKm2: number
}

interface RoundGuess {
  guessedLat: number
  guessedLng: number
  actualLat: number
  actualLng: number
  distanceKm: number
  score: number
}

interface GameState {
  cardId: number
  currentRoundIndex: number
  status: 'playing' | 'guessed' | 'finished'
  guesses: (RoundGuess | null)[]
}

const payload = computed(() => props.card.payload as unknown as EstimatrPayload)
const rounds = computed(() => payload.value?.rounds || [])
const areaKm2 = computed(() => payload.value?.areaKm2 || 0)

// Game State persisted via local storage
const gameState = useObjStorage<GameState>(`estimatr_state_${props.card.id}`, {
  cardId: props.card.id,
  currentRoundIndex: 0,
  status: 'playing',
  guesses: [],
})

if (!gameState.value || gameState.value.cardId !== props.card.id) {
  gameState.value = {
    cardId: props.card.id,
    currentRoundIndex: 0,
    status: 'playing',
    guesses: Array(rounds.value.length).fill(null),
  }
}

// User active marker guess coordinates
const tempGuess = ref<{ lat: number; lng: number } | null>(null)

// Map parameters and reactive drag-resize coordinates
const mapInstance = ref<maplibregl.Map | null>(null)
const mapStyle = ref<StyleName>('LIBERTY')

function requireMap(): maplibregl.Map {
  return mapInstance.value as unknown as maplibregl.Map
}

const mapWidth = ref(440) // Generous starting map size
const mapHeight = ref(330)

const minWidth = 320
const maxWidth = 800
const minHeight = 240
const maxHeight = 600

let isResizing = false
let startX = 0
let startY = 0
let startWidth = 0
let startHeight = 0

// Stable map options to prevent viewport resets when switching style layers
const stableMapOptions = {
  center: { lon: 5, lat: 20 },
  zoom: 0,
  attributionControl: {
    compact: true,
  },
}

// Marker handles
let guessMarker: maplibregl.Marker | null = null
let actualMarker: maplibregl.Marker | null = null
const summaryMarkers: maplibregl.Marker[] = []
const summaryLineLayers: string[] = []

const currentRound = computed<EstimatrRound | null>(() => {
  if (rounds.value.length === 0) return null
  return rounds.value[gameState.value.currentRoundIndex] || null
})

const currentMediaItem = computed(() => currentRound.value?.mediaItem || null)

const totalScore = computed(() => {
  return gameState.value.guesses.reduce((sum, g) => sum + (g?.score || 0), 0)
})

const currentRoundGuess = computed(() => {
  return gameState.value.guesses[gameState.value.currentRoundIndex]
})

// Geodesic Interpolation helper for curved lines (Great Circle paths)
function getGreatCircleRoute(
  startLat: number,
  startLng: number,
  endLat: number,
  endLng: number,
  pointsCount = 60,
): [number, number][] {
  const coords: [number, number][] = []

  const lat1 = (startLat * Math.PI) / 180
  const lon1 = (startLng * Math.PI) / 180
  const lat2 = (endLat * Math.PI) / 180
  const lon2 = (endLng * Math.PI) / 180

  const dLon = lon2 - lon1
  const dLat = lat2 - lat1

  const a =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos(lat1) * Math.cos(lat2) * Math.sin(dLon / 2) * Math.sin(dLon / 2)
  const g = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a))

  if (g === 0) {
    return [
      [startLng, startLat],
      [endLng, endLat],
    ]
  }

  for (let i = 0; i <= pointsCount; i++) {
    const f = i / pointsCount
    const A = Math.sin((1 - f) * g) / Math.sin(g)
    const B = Math.sin(f * g) / Math.sin(g)

    const x = A * Math.cos(lat1) * Math.cos(lon1) + B * Math.cos(lat2) * Math.cos(lon2)
    const y = A * Math.cos(lat1) * Math.sin(lon1) + B * Math.cos(lat2) * Math.sin(lon2)
    const z = A * Math.sin(lat1) + B * Math.sin(lat2)

    const lat = Math.atan2(z, Math.sqrt(x * x + y * y))
    const lon = Math.atan2(y, x)

    coords.push([(lon * 180) / Math.PI, (lat * 180) / Math.PI])
  }

  return coords
}

// Drag Resizing Logic for Bottom-Left Anchored Map
function startResize(e: MouseEvent) {
  isResizing = true
  startX = e.clientX
  startY = e.clientY
  startWidth = mapWidth.value
  startHeight = mapHeight.value

  document.addEventListener('mousemove', handleResize)
  document.addEventListener('mouseup', stopResize)
  e.preventDefault()
}

function handleResize(e: MouseEvent) {
  if (!isResizing) return
  const dx = e.clientX - startX
  const dy = startY - e.clientY

  mapWidth.value = Math.max(minWidth, Math.min(maxWidth, startWidth + dx))
  mapHeight.value = Math.max(minHeight, Math.min(maxHeight, startHeight + dy))

  if (mapInstance.value) {
    mapInstance.value.resize()
  }
}

function stopResize() {
  isResizing = false
  document.removeEventListener('mousemove', handleResize)
  document.removeEventListener('mouseup', stopResize)
}

function startResizeTouch(e: TouchEvent) {
  if (e.touches.length !== 1) return
  isResizing = true
  startX = e.touches[0].clientX
  startY = e.touches[0].clientY
  startWidth = mapWidth.value
  startHeight = mapHeight.value

  document.addEventListener('touchmove', handleResizeTouch)
  document.addEventListener('touchend', stopResizeTouch)
  e.preventDefault()
}

function handleResizeTouch(e: TouchEvent) {
  if (!isResizing || e.touches.length !== 1) return
  const dx = e.touches[0].clientX - startX
  const dy = startY - e.touches[0].clientY

  mapWidth.value = Math.max(minWidth, Math.min(maxWidth, startWidth + dx))
  mapHeight.value = Math.max(minHeight, Math.min(maxHeight, startHeight + dy))

  if (mapInstance.value) {
    mapInstance.value.resize()
  }
}

function stopResizeTouch() {
  isResizing = false
  document.removeEventListener('touchmove', handleResizeTouch)
  document.removeEventListener('touchend', stopResizeTouch)
}

// Distance & Score Calculations
function haversineDistance(lat1: number, lon1: number, lat2: number, lon2: number): number {
  const R = 6371
  const dLat = ((lat2 - lat1) * Math.PI) / 180
  const dLon = ((lon2 - lon1) * Math.PI) / 180
  const a =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos((lat1 * Math.PI) / 180) *
      Math.cos((lat2 * Math.PI) / 180) *
      Math.sin(dLon / 2) *
      Math.sin(dLon / 2)
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a))
  return R * c
}

function calculateScore(distanceKm: number, areaKm2Value: number): number {
  let k = 2000
  if (areaKm2Value && areaKm2Value > 0) {
    k = Math.max(1, Math.sqrt(areaKm2Value) * 0.15)
  }
  const score = Math.round(5000 * Math.exp(-distanceKm / k))
  return Math.min(5000, Math.max(0, score))
}

function formatDistance(distKm: number): string {
  if (distKm < 1) {
    return `${Math.round(distKm * 1000)} m`
  }
  return `${distKm.toFixed(1)} km`
}

// Map Fitting bounds safety calculator (prevents crashes from extreme padding / small canvas)
function safeFitBounds(bounds: maplibregl.LngLatBounds, isFinishedState = false) {
  if (!mapInstance.value) return
  const map = mapInstance.value
  const canvas = map.getCanvas()
  if (!canvas) return

  const width = canvas.clientWidth
  const height = canvas.clientHeight

  if (isFinishedState) {
    // Finished state: Full-screen canvas. Use horizontal offset to avoid left-panel overlap
    const leftPad = width > 800 ? 480 : Math.round(width * 0.1)
    const rightPad = Math.round(width * 0.1)
    const topPad = Math.round(height * 0.12)
    const bottomPad = Math.round(height * 0.12)

    map.fitBounds(bounds, {
      padding: { top: topPad, bottom: bottomPad, left: leftPad, right: rightPad },
      maxZoom: 10,
      duration: 1200,
    })
  } else {
    // In-game state: Compact map container size. Keep padding safe and proportional
    const hPad = Math.min(50, Math.round(width * 0.12))
    const vPad = Math.min(50, Math.round(height * 0.12))

    map.fitBounds(bounds, {
      padding: { top: vPad, bottom: vPad, left: hPad, right: hPad },
      maxZoom: 14,
      duration: 1000,
    })
  }
}

// Map Loading & Drawing Trigger handlers
function handleMapLoad(loadedMap: maplibregl.Map) {
  mapInstance.value = loadedMap

  loadedMap.on('click', (e) => {
    if (gameState.value.status !== 'playing') return
    const { lng, lat } = e.lngLat
    tempGuess.value = { lat, lng }
    updateGuessMarker(lat, lng)
  })

  nextTick(() => {
    drawAllOnMap()
  })
}

function handleStyleLoad() {
  drawAllOnMap()
}

function toggleMapStyle() {
  mapStyle.value = mapStyle.value === 'LIBERTY' ? 'SATELLITE' : 'LIBERTY'
}

function updateGuessMarker(lat: number, lng: number) {
  if (!mapInstance.value) return

  if (!guessMarker) {
    const el = document.createElement('div')
    el.className = 'guess-pin-marker'
    el.innerHTML = `
      <div class="pin-ring"></div>
      <div class="pin-dot"></div>
    `
    guessMarker = new maplibregl.Marker({
      element: el,
      anchor: 'center',
    })
      .setLngLat([lng, lat])
      .addTo(requireMap())
  } else {
    guessMarker.setLngLat([lng, lat])
  }
}

function updateActualMarker(lat: number, lng: number) {
  if (!mapInstance.value || !currentMediaItem.value) return

  const thumbUrl = mediaItemService.getPhotoThumbnail(
    currentMediaItem.value.id,
    144,
    !currentMediaItem.value.hasThumbnails,
  )

  if (!actualMarker) {
    const el = document.createElement('div')
    el.className = 'actual-pin-marker'

    const circle = document.createElement('div')
    circle.className = 'marker-circle'
    circle.style.backgroundImage = `url(${thumbUrl})`

    const triangle = document.createElement('div')
    triangle.className = 'marker-triangle'

    el.appendChild(circle)
    el.appendChild(triangle)

    // offset [0, -12] lifts the pin up so guess marker underneath is clearly visible
    actualMarker = new maplibregl.Marker({
      element: el,
      anchor: 'bottom',
      offset: [0, -25],
    })
      .setLngLat([lng, lat])
      .addTo(requireMap())
  } else {
    actualMarker.setLngLat([lng, lat])
    const circle = actualMarker.getElement().querySelector('.marker-circle') as HTMLElement
    if (circle) {
      circle.style.backgroundImage = `url(${thumbUrl})`
    }
  }
}

function drawDottedLine(coord1: [number, number], coord2: [number, number]) {
  if (!mapInstance.value) return
  const map = mapInstance.value

  const curvedCoordinates = getGreatCircleRoute(coord1[1], coord1[0], coord2[1], coord2[0])

  const geojson: GeoJSON.Feature<GeoJSON.LineString> = {
    type: 'Feature',
    properties: {},
    geometry: {
      type: 'LineString',
      coordinates: curvedCoordinates,
    },
  }

  if (map.getSource('route')) {
    ;(map.getSource('route') as maplibregl.GeoJSONSource).setData(geojson)
  } else {
    map.addSource('route', {
      type: 'geojson',
      data: geojson,
    })

    map.addLayer({
      id: 'route',
      type: 'line',
      source: 'route',
      layout: {
        'line-join': 'round',
        'line-cap': 'round',
      },
      paint: {
        'line-color': '#ff9f0a',
        'line-width': 3.5,
        'line-dasharray': [2, 2],
      },
    })
  }
}

function clearMapDrawings() {
  if (guessMarker) {
    guessMarker.remove()
    guessMarker = null
  }
  if (actualMarker) {
    actualMarker.remove()
    actualMarker = null
  }

  summaryMarkers.forEach((m) => m.remove())
  summaryMarkers.length = 0

  if (mapInstance.value) {
    const map = mapInstance.value
    if (map.style) {
      if (map.getLayer('route')) map.removeLayer('route')
      if (map.getSource('route')) map.removeSource('route')

      summaryLineLayers.forEach((lineId) => {
        if (map.getLayer(lineId)) map.removeLayer(lineId)
        if (map.getSource(lineId)) map.removeSource(lineId)
      })
    }
    summaryLineLayers.length = 0
  }
}

function drawAllOnMap() {
  if (!mapInstance.value) return
  const map = requireMap()

  clearMapDrawings()

  if (gameState.value.status === 'playing') {
    if (tempGuess.value) {
      updateGuessMarker(tempGuess.value.lat, tempGuess.value.lng)
    }
  } else if (gameState.value.status === 'guessed') {
    const currentGuess = currentRoundGuess.value
    if (currentGuess) {
      updateGuessMarker(currentGuess.guessedLat, currentGuess.guessedLng)
      updateActualMarker(currentGuess.actualLat, currentGuess.actualLng)
      drawDottedLine(
        [currentGuess.guessedLng, currentGuess.guessedLat],
        [currentGuess.actualLng, currentGuess.actualLat],
      )

      const bounds = new maplibregl.LngLatBounds()
      bounds.extend([currentGuess.guessedLng, currentGuess.guessedLat])
      bounds.extend([currentGuess.actualLng, currentGuess.actualLat])
      map.fitBounds(bounds, { padding: 80, maxZoom: 14, duration: 1000 })
    }
  } else if (gameState.value.status === 'finished') {
    const bounds = new maplibregl.LngLatBounds()
    let validGuesses = 0

    gameState.value.guesses.forEach((g, idx) => {
      if (!g) return
      validGuesses++

      const gEl = document.createElement('div')
      gEl.className = 'summary-marker guess-summary'
      gEl.innerHTML = `<span class="marker-label">${idx + 1}</span>`
      const gMarker = new maplibregl.Marker({ element: gEl, anchor: 'center' })
        .setLngLat([g.guessedLng, g.guessedLat])
        .addTo(map)
      summaryMarkers.push(gMarker)
      bounds.extend([g.guessedLng, g.guessedLat])

      const aEl = document.createElement('div')
      aEl.className = 'summary-marker actual-summary'
      aEl.innerHTML = `<span class="marker-label">${idx + 1}</span>`
      const aMarker = new maplibregl.Marker({ element: aEl, anchor: 'center' })
        .setLngLat([g.actualLng, g.actualLat])
        .addTo(map)
      summaryMarkers.push(aMarker)
      bounds.extend([g.actualLng, g.actualLat])

      // Great Circle curved connectors on final view
      const lineId = `line-summary-${idx}`
      const curveCoords = getGreatCircleRoute(g.guessedLat, g.guessedLng, g.actualLat, g.actualLng)

      map.addSource(lineId, {
        type: 'geojson',
        data: {
          type: 'Feature',
          properties: {},
          geometry: {
            type: 'LineString',
            coordinates: curveCoords,
          },
        },
      })
      map.addLayer({
        id: lineId,
        type: 'line',
        source: lineId,
        layout: {
          'line-join': 'round',
          'line-cap': 'round',
        },
        paint: {
          'line-color': '#30d158',
          'line-width': 2.5,
          'line-dasharray': [2, 2],
        },
      })
      summaryLineLayers.push(lineId)
    })

    if (validGuesses > 0) {
      nextTick(() => {
        safeFitBounds(bounds, true)
      })
    }
  }
}

function handleGuessClick() {
  if (!tempGuess.value || !currentRound.value) return

  const actualLat = currentRound.value.latitude
  const actualLng = currentRound.value.longitude
  const guessedLat = tempGuess.value.lat
  const guessedLng = tempGuess.value.lng

  const distance = haversineDistance(guessedLat, guessedLng, actualLat, actualLng)
  const score = calculateScore(distance, areaKm2.value)

  const updatedGuesses = [...gameState.value.guesses]
  updatedGuesses[gameState.value.currentRoundIndex] = {
    guessedLat,
    guessedLng,
    actualLat,
    actualLng,
    distanceKm: distance,
    score,
  }

  gameState.value = {
    ...gameState.value,
    guesses: updatedGuesses,
    status: 'guessed',
  }

  drawAllOnMap()
}

function nextRound() {
  const nextIdx = gameState.value.currentRoundIndex + 1
  if (nextIdx < rounds.value.length) {
    gameState.value = {
      ...gameState.value,
      currentRoundIndex: nextIdx,
      status: 'playing',
    }
    tempGuess.value = null
    clearMapDrawings()

    if (mapInstance.value) {
      mapInstance.value.setZoom(0)
      mapInstance.value.setCenter([5, 20])
    }
  } else {
    gameState.value = {
      ...gameState.value,
      status: 'finished',
    }
    cardStore.completedCards.push(props.card.id)
    drawAllOnMap()
  }
}

function goBack() {
  router.push('/')
}

watch(
  currentMediaItem,
  () => {
    if (!currentMediaItem.value) return
    mediaItemStore.fetchMediaItem(currentMediaItem.value.id)
  },
  { immediate: true },
)

watch(
  () => gameState.value.currentRoundIndex,
  () => {
    tempGuess.value = null
    nextTick(() => {
      drawAllOnMap()
    })
  },
)

onUnmounted(() => {
  try {
    clearMapDrawings()
  } catch (err) {
    console.error('Error clearing drawings on unmount:', err)
  }

  if (mapInstance.value) {
    try {
      mapInstance.value.remove()
    } catch (err) {
      console.error('Error removing map instance:', err)
    }
    mapInstance.value = null
  }
})
</script>

<template>
  <div class="estimatr-container" :class="{ 'finished-mode': gameState.status === 'finished' }">
    <!-- Top HUD Bar -->
    <div class="estimatr-header">
      <div class="header-left">
        <v-btn icon="mdi-close" variant="text" color="on-surface" size="small" @click="goBack" />
        <div class="header-titles">
          <h3>{{ card.title }}</h3>
          <p class="subtitle" v-if="card.subtitle">{{ card.subtitle }}</p>
        </div>
      </div>

      <div class="header-center" v-if="gameState.status !== 'finished'">
        <v-chip color="primary" variant="flat" class="round-chip">
          Round {{ gameState.currentRoundIndex + 1 }} / {{ rounds.length }}
        </v-chip>
      </div>

      <div class="header-right">
        <div class="score-display">
          <span class="label">Total Score</span>
          <span class="value">{{ totalScore }}</span>
        </div>
      </div>
    </div>

    <!-- Active Main Screen Presentation -->
    <div class="estimatr-main-content" v-if="gameState.status !== 'finished'">
      <!-- Media Player Viewer Box -->
      <div class="media-viewport">
        <media-viewer
          class="media-frame"
          v-if="currentMediaItem"
          :media-item-id="currentMediaItem.id"
          :disable-event-capture="false"
          :show-ui="true"
          :is-video="currentMediaItem.isVideo"
          :muted="false"
        />
      </div>

      <!-- Corner Map Placement Overlay -->
      <div
        class="map-wrapper"
        :style="{
          width: `${mapWidth}px`,
          height: `${mapHeight}px`,
          transform: currentMediaItem?.isVideo ? `translate(10px, -115px)` : '',
        }"
      >
        <div class="map-inner-container">
          <base-map
            class="base-map"
            :map-style="mapStyle"
            :map-options="stableMapOptions"
            @load="handleMapLoad"
            @style-load="handleStyleLoad"
          />

          <!-- Custom Resizer drag Handle Anchor -->
          <div
            class="resize-handle"
            @mousedown="startResize"
            @touchstart="startResizeTouch"
            title="Drag to resize map"
          >
            <v-icon size="14" color="on-surface">mdi-resize-bottom-right</v-icon>
          </div>

          <!-- Floating Map Style controls -->
          <div class="map-floating-controls">
            <v-btn
              color="primary"
              density="compact"
              elevation="3"
              :icon="mapStyle === 'SATELLITE' ? 'mdi-map' : 'mdi-earth'"
              @click="toggleMapStyle"
            />
          </div>

          <!-- Bottom Action Confirmation Board inside Map Area -->
          <div class="map-bottom-actions">
            <template v-if="gameState.status === 'playing'">
              <v-btn
                block
                color="primary"
                rounded="lg"
                height="44"
                :disabled="!tempGuess"
                @click="handleGuessClick"
              >
                {{ tempGuess ? 'Make Guess' : 'Place Marker On Map' }}
              </v-btn>
            </template>
            <template v-else-if="gameState.status === 'guessed'">
              <v-btn block color="success" rounded="lg" height="44" @click="nextRound">
                {{
                  gameState.currentRoundIndex + 1 === rounds.length ? 'View Results' : 'Next Round'
                }}
              </v-btn>
            </template>
          </div>
        </div>
      </div>

      <!-- Banner notification overlay for results of current round -->
      <transition name="slide-up">
        <div class="round-result-card" v-if="gameState.status === 'guessed' && currentRoundGuess">
          <div class="result-box">
            <div class="result-metric">
              <span class="metric-label">Distance</span>
              <span class="metric-value warning-color">{{
                formatDistance(currentRoundGuess.distanceKm)
              }}</span>
            </div>
            <v-divider vertical class="result-divider" />
            <div class="result-metric">
              <span class="metric-label">Score</span>
              <span class="metric-value success-color">+{{ currentRoundGuess.score }}</span>
            </div>
          </div>
          <v-progress-linear
            color="success"
            height="6"
            rounded
            class="result-progress"
            :model-value="(currentRoundGuess.score / 5000) * 100"
          />
        </div>
      </transition>
    </div>

    <!-- Overall Completed Game Summary Screen Mode -->
    <div class="estimatr-summary-content" v-else>
      <!-- Background Map takes full screen -->
      <div class="full-summary-map-container">
        <base-map
          class="base-map"
          :map-style="mapStyle"
          :map-options="stableMapOptions"
          @load="handleMapLoad"
          @style-load="handleStyleLoad"
        />
        <div class="map-floating-controls">
          <v-btn
            size="x-small"
            color="surface"
            elevation="3"
            :icon="mapStyle === 'SATELLITE' ? 'mdi-map' : 'mdi-earth'"
            @click="toggleMapStyle"
          />
        </div>
      </div>

      <!-- Floating summary statistics panel on left hand side -->
      <div class="summary-details-panel">
        <v-card class="summary-details-card" flat border>
          <div class="summary-card-header">
            <v-icon size="48" color="success" class="success-icon">mdi-check-circle-outline</v-icon>
            <h2>Challenge Completed!</h2>
            <p class="summary-desc">Come back tomorrow for another game.</p>
          </div>

          <!-- Total Score Ring representation -->
          <v-progress-circular
            class="score-circle-container"
            size="150"
            width="7"
            :model-value="(totalScore / (rounds.length * 5000)) * 100"
            color="success"
          >
            <div class="score-ring">
              <span class="score-num">{{ totalScore }}</span>
              <span class="score-label">Out of {{ (rounds.length * 5000).toLocaleString() }}</span>
            </div>
          </v-progress-circular>

          <!-- Individual Rounds Breakdowns lists -->
          <div class="rounds-summary-list">
            <a
              v-for="(guess, index) in gameState.guesses.filter((g) => g)"
              :key="index"
              class="round-row"
              :href="`/view/${rounds[index].mediaItem.id}`"
              target="_blank"
            >
              <div class="round-number">
                <v-avatar color="primary" size="26" class="round-avatar">
                  {{ index + 1 }}
                </v-avatar>
              </div>
              <div class="round-thumbnail-wrapper">
                <img
                  v-if="rounds[index]"
                  class="round-thumb"
                  :src="
                    mediaItemService.getPhotoThumbnail(
                      rounds[index].mediaItem.id,
                      144,
                      !rounds[index].mediaItem.hasThumbnails,
                    )
                  "
                />
              </div>
              <template v-if="guess">
                <div class="round-data">
                  <div class="round-caption">Round {{ index + 1 }}</div>
                  <div class="round-distance">{{ formatDistance(guess.distanceKm) }} away</div>
                </div>
                <div class="round-pts success-color">+{{ guess.score }}</div>
              </template>
            </a>
          </div>

          <!-- Bottom Action Row -->
          <div class="summary-actions">
            <v-btn block color="primary" variant="tonal" size="large" rounded="lg" @click="goBack">
              Return Home
            </v-btn>
          </div>
        </v-card>
      </div>
    </div>
  </div>
</template>

<style scoped>
.estimatr-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgb(var(--v-theme-background));
  color: rgb(var(--v-theme-on-background));
  display: flex;
  flex-direction: column;
  overflow: hidden;
  font-family: inherit;
}

/* Header HUD Styling */
.estimatr-header {
  height: 72px;
  background-color: rgba(var(--v-theme-surface-container-low), 0.85);
  backdrop-filter: saturate(150%) blur(12px);
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  z-index: 101;
}

.header-left {
  display: flex;
  align-items: center;
}

.header-titles {
  margin-left: 16px;
}

.header-titles h3 {
  margin: 0;
  font-size: 1.15rem;
  font-weight: 600;
  color: rgb(var(--v-theme-on-surface));
}

.header-titles .subtitle {
  margin: 0;
  font-size: 0.85rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.header-center {
  display: flex;
  align-items: center;
}

.round-chip {
  font-weight: 700;
  padding: 16px 20px !important;
}

.score-display {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.score-display .label {
  font-size: 0.75rem;
  text-transform: uppercase;
  color: rgb(var(--v-theme-on-surface-variant));
  letter-spacing: 0.5px;
}

.score-display .value {
  font-size: 1.3rem;
  font-weight: 700;
  color: rgb(var(--v-theme-primary));
}

/* Playing Mode Screen Framework */
.estimatr-main-content {
  flex-grow: 1;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: calc(100% - 72px);
}

.media-viewport {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1;
  padding: 24px;
}

.media-frame {
  width: 100%;
  height: 100%;
  max-height: calc(100% - 40px);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 24px; /* very rounded corners */
  overflow: hidden;
  box-shadow: 0 12px 36px rgba(0, 0, 0, 0.15);
  background-color: rgb(var(--v-theme-surface-container-low));
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.media-content {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

/* Draggable Map Wrapper Base Box */
.map-wrapper {
  position: absolute;
  bottom: 24px;
  left: 24px;
  z-index: 10;
  border-radius: 24px; /* very rounded corners */
  overflow: hidden;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15);
  background-color: rgb(var(--v-theme-surface-container-high));
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  box-sizing: border-box;
}

.map-inner-container {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.base-map {
  flex-grow: 1;
  width: 100%;
  height: 100%;
}

/* Top Right Corner Resizer Handle style */
.resize-handle {
  position: absolute;
  top: 0;
  right: 0;
  width: 28px;
  height: 28px;
  background-color: rgb(var(--v-theme-surface-container-high));
  border-bottom-left-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: ne-resize;
  z-index: 15;
  border-left: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  box-shadow: -2px 2px 6px rgba(0, 0, 0, 0.1);
  touch-action: none;
  transition: background-color 0.2s;
}

.resize-handle:hover {
  background-color: rgb(var(--v-theme-primary));
}

.resize-handle:hover .v-icon {
  color: rgb(var(--v-theme-on-primary)) !important;
}

.map-floating-controls {
  position: absolute;
  top: 36px;
  right: 8px;
  display: flex;
  flex-direction: column;
  z-index: 5;
  gap: 8px;
}

.map-bottom-actions {
  padding: 12px 16px;
  background-color: rgb(var(--v-theme-surface-container-high));
  border-top: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

/* Floating Banner showing round accuracy statistics */
.round-result-card {
  position: absolute;
  bottom: 24px;
  right: 24px;
  width: 340px;
  background-color: rgba(var(--v-theme-surface-container-high), 0.85);
  backdrop-filter: saturate(150%) blur(12px);
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 24px; /* very rounded corners */
  padding: 16px;
  z-index: 10;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15);
}

.result-box {
  display: flex;
  align-items: center;
  justify-content: space-around;
}

.result-metric {
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 1;
}

.metric-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.8;
  margin-bottom: 2px;
}

.metric-value {
  font-size: 1.25rem;
  font-weight: 700;
}

.warning-color {
  color: rgb(var(--v-theme-warning)) !important;
}

.success-color {
  color: rgb(var(--v-theme-success)) !important;
}

.result-divider {
  margin: 0 16px;
  opacity: 0.3;
}

.result-progress {
  margin-top: 12px;
}

/* Game Completed Summary mode Layout CSS */
.estimatr-summary-content {
  flex-grow: 1;
  position: relative;
  display: flex;
  width: 100%;
  height: calc(100% - 72px);
}

.full-summary-map-container {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 1;
}

.summary-details-panel {
  position: absolute;
  top: 24px;
  left: 24px;
  bottom: 24px;
  z-index: 2;
  display: flex;
}

.summary-details-card {
  background-color: rgba(var(--v-theme-surface-container-high), 0.85) !important;
  backdrop-filter: saturate(150%) blur(12px);
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity)) !important;
  border-radius: 24px !important; /* very rounded corners */
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  max-height: 100%;
  padding: 24px;
  width: 440px;
}

.summary-card-header {
  text-align: center;
  margin-bottom: 24px;
}

.success-icon {
  margin-bottom: 8px;
}

.summary-card-header h2 {
  font-size: 1.5rem;
  font-weight: 700;
  margin-top: 8px;
  color: rgb(var(--v-theme-on-surface));
}

.summary-desc {
  font-size: 0.875rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.score-circle-container {
  display: flex;
  justify-content: center;
  margin: 30px auto;
  margin-top: 0;
}

.score-ring {
  width: 130px;
  height: 130px;
  border-radius: 50%;
  border: 4px solid rgb(var(--v-theme-success));
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.score-num {
  font-size: 2.1rem;
  font-weight: 800;
  color: rgb(var(--v-theme-success));
  line-height: 1;
}

.score-label {
  font-size: 0.75rem;
  letter-spacing: 0.5px;
  color: rgb(var(--v-theme-on-surface-variant));
  opacity: 0.8;
}

.rounds-summary-list {
  flex-grow: 1;
  margin-bottom: 24px;
}

.round-row {
  display: flex;
  align-items: center;
  background-color: rgba(var(--v-theme-on-surface), 0.04);
  border-radius: 16px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  padding: 12px;
  margin-bottom: 8px;
  transition:
    transform 0.2s,
    background-color 0.2s;
  text-decoration: none;
}

.round-row:hover {
  transform: translateX(4px);
  background-color: rgba(var(--v-theme-on-surface), 0.08);
}

.round-avatar {
  font-size: 0.85rem;
  font-weight: 700;
}

.round-thumbnail-wrapper {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  overflow: hidden;
  background-color: rgb(var(--v-theme-surface-container-low));
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 12px;
}

.round-thumb {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.round-data {
  flex-grow: 1;
}

.round-caption {
  font-size: 0.75rem;
  color: rgb(var(--v-theme-on-surface-variant));
}

.round-distance {
  font-size: 0.875rem;
  font-weight: 700;
  color: rgb(var(--v-theme-on-surface));
}

.round-pts {
  font-weight: 700;
  font-size: 1rem;
}

/* Animations for result cards transitions */
.slide-up-enter-active,
.slide-up-leave-active {
  transition: all 0.35s cubic-bezier(0.25, 0.8, 0.25, 1);
}
.slide-up-enter-from,
.slide-up-leave-to {
  transform: translateY(100px);
  opacity: 0;
}

/* Mobile responsive scaling overrides */
@media (max-width: 960px) {
  .estimatr-summary-content {
    flex-direction: column-reverse;
  }
  .full-summary-map-container {
    height: 50%;
    top: auto;
    bottom: 0;
  }
  .summary-details-panel {
    top: 10px;
    left: 10px;
    right: 10px;
    bottom: calc(50% + 10px);
    width: auto;
  }
}

@media (max-width: 600px) {
  .map-wrapper {
    width: 280px !important;
    height: 220px !important;
    bottom: 12px;
    left: 12px;
  }
  .round-result-card {
    width: calc(100% - 24px);
    bottom: 12px;
    right: 12px;
  }
}
</style>

<style>
/*
  For generated MapLibre GL Markers
*/
.actual-pin-marker {
  width: 52px;
  height: 60px;
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  filter: drop-shadow(0 4px 6px rgba(0, 0, 0, 0.2));
  pointer-events: none;
  box-sizing: border-box;
}

.marker-circle {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  border: 2px solid rgb(var(--v-theme-success));
  background-color: rgba(var(--v-theme-surface), 0.65);
  background-position: center;
  background-repeat: no-repeat;
  background-size: cover;
  box-sizing: border-box;
  box-shadow: 0 0 0 4px rgba(var(--v-theme-success), 0.25);
}

.marker-triangle {
  width: 0;
  height: 0;
  border-left: 6px solid transparent;
  border-right: 6px solid transparent;
  border-top: 8px solid rgb(var(--v-theme-success));
  margin-top: -1px;
}

/* Custom Interactive Guess Marker representation */
.guess-pin-marker {
  width: 24px;
  height: 24px;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.pin-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  border: 2px solid orange;
  background-color: rgb(255 144 0 / 0.37);
  animation: markerPulse 1.5s infinite ease-in-out;
}

.pin-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background-color: #ffe7b9;
  border: 1px solid #ff6b02;
}

@keyframes markerPulse {
  0% {
    transform: scale(0.8);
    opacity: 1;
  }
  100% {
    transform: scale(1.6);
    opacity: 0;
  }
}

/* Game summary pinpoint stylings */
.summary-marker {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  font-size: 0.8rem;
  color: #ffffff !important;
  border: 2px solid #ffffff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  box-sizing: border-box;
}

.guess-summary {
  background-color: #c88202;
}

.actual-summary {
  background-color: green;
}

.marker-label {
  line-height: 1;
  color: #ffffff !important;
}
</style>
